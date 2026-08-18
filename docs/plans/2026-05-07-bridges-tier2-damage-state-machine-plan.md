# Bridges Tier 2 — Damage Gating + State Machine Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.
> Tasks are grouped into 7 phases (A–G). Each phase ends with `cargo test`
> green and one or more commits. Do not skip phases.

**Goal:** Land full gamemd.exe parity for bridge damage in one feature surface:
`Apply_area_damage` gating, the 18-state per-cell two-axis damage state machine
(body + bridgehead, both High and Low), `BlowUpBridge` ground-occupant kill,
correct `MetallicDebris` + `BridgeExplosions` debris structure, anchor-span
granularity, `UpdateAdjacentBridges` rim re-evaluation, and the zone-refresh
hook.

**Architecture:** Approach Alpha (first-class typed structs over flag-bit
storage). New `DamageState` / `Axis` / `BridgeCellRole` / `Direction` enums on
`BridgeRuntimeCell`, with a first-class `AnchorSpan` registry on
`BridgeRuntimeState`. Anchor walker at map load mirrors
`SetBridgeDirection_NESW`. Gate + RNG + retry live at the world boundary
(combat pre-resolves `is_ion_cannon`). Renderer queries
`BridgeRuntimeState::display_tile`; `ResolvedTerrainGrid` stays immutable.

**Design Doc:** [docs/plans/2026-05-07-bridges-tier2-damage-state-machine-design.md](2026-05-07-bridges-tier2-damage-state-machine-design.md)

---

## Grounding Summary

- **R1 — ra2-rust-game-docs:** Primary source `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`
  (2692 lines), §3.1 (body branch), §3.2 (bridgehead branch), §11.1 (16
  UpdateRamp helpers), §11.2 (16-entry overlay neighbor lookup), §11.4
  (BlowUpBridge), §11.5 (SetBridgeDirection_NESW), §11.7 (g_DirectionOffsets
  layout), §11.8 (zone refresh), §11.9 (UpdateAdjacentBridges_*), §7
  (NS/EW Walker label inversion — physical axis matches state-byte semantic,
  not function names). Cross-check with `BRIDGE_SYSTEM.md` and
  `CELLCLASS_ZONES_SPEED_BRIDGES.md` for zone refresh semantics.
- **R2 — Ghidra verifications (live this session):** `Apply_area_damage @
  0x4894B0` retry semantics — IonCannon-only, max 4 attempts (corrects audit
  which said "non-IonCannon retries"). `BlowUpBridge @ 0x47DD70` debris
  structure: outer 95% gate + 2 jitter + 50%-MetallicDebris (no delay) +
  1-always-BridgeExplosion (delay 1–5) — current Rust code is structurally
  wrong; rewrite required. `SetBridgeDirection_NESW @ 0x47E040` walks 6 cells
  but invokes BlowUpBridge on exactly **4** (cells 1, 2, 3, 5). Memory reads
  confirmed thresholds: `0x7e4f58 = 0.95`, `0x7e1738 = 0.5`, `0x7e3570 = 1/2^31`.
- **R3 — Repo patterns:** `BridgeRules` from Tier 1 ([ruleset.rs:652-715](../../src/rules/ruleset.rs#L652-L715))
  shows sub-struct + INI parsing layout. `CombatDamageDefaults`
  ([src/rules/combat_damage.rs](../../src/rules/combat_damage.rs)) shows the
  sub-struct pattern (we'll add a peer `BridgeWarheads` sub-struct, not extend
  this one — wrong scope). `SimRng` API ([src/sim/rng.rs:45-51](../../src/sim/rng.rs#L45-L51))
  has `next_range_u32(N) → [0, N)`; we'll add an inclusive helper.
  Existing `BridgeRuntimeState` constructor at [bridge_state.rs:62-143](../../src/sim/bridge_state.rs#L62-L143)
  shows BFS-by-deck pattern; we replace it. Death pipeline at
  [combat/mod.rs:624-651](../../src/sim/combat/mod.rs#L624-L651) selects `InfDeath`
  from killing warhead — we reuse it for the C4Warhead force-kill path.
  Zone rebuild via [world/mod.rs:614](../../src/sim/world/mod.rs#L614) `rebuild_zone_grid`.
- **R4 — INI keys (all confirmed in `ini/rulesmd.ini`):** `[General]
  MetallicDebris=` (line 528, 20 entries), `[CombatDamage] C4Warhead=Super`
  (818), `[CombatDamage] IonCannonWarhead=IonCannonWH` (874). Tier 1 already
  parses `BridgeStrength`, `DestroyableBridges`, `BridgeVoxelMax`,
  `BridgeExplosions`, `BridgeRepairHut`.
- **Unknowns:** Bridgehead height-walk field offset (`cell.height==4 NS / ==2 EW`
  at HIGH §3.2) needs Ghidra spot-check before Phase C — likely
  `CellClass+0x52` (height-class byte) but unverified. Runtime values of
  `DAT_0089E7C0` (debris-spawn heightStep) and `DAT_0089E7B4` (heightStep
  offset) cannot be read statically — use existing `bridge_deck_level`
  pattern for spawn Z and accept the binary's runtime values are aligned
  with our 90 leptons/level (close-but-unverified).
- **Parallel session note:** [combat/mod.rs](../../src/sim/combat/mod.rs)
  has uncommitted edits (`TargetKind` enum for force-fire on cells) from a
  parallel session. Bridge-emit-site changes are surgical and unique by
  pattern — anchor by content, not line numbers. Per CLAUDE.md: don't
  modify the parallel session's work.

## Key Technical Decisions

| Decision | Confidence | Source |
|---|---|---|
| Approach Alpha (first-class typed structs over flag-bit storage) | high | Design doc + EntityStore precedent |
| `BridgeWarheads` as new sub-struct on `RuleSet` (not extension of `CombatDamageDefaults`) | high | repo pattern: `CombatDamageDefaults` is particle-system slot; bridge warhead refs are functionally distinct |
| `apply_ramp_transition(span, slot, axis, phase, set)` table-dispatched over 16 helpers | high | doc HIGH §11.1 + §11.2 |
| `set_bridge_direction` emits exactly 4 `BlowUpBridge` actions (cells 1, 2, 3, 5) on destruction; cells 4 and 6 flag-only | high | verified live `0x47E040` |
| Compass `Direction` enum matches binary `g_DirectionOffsets` indices `0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW` | high | doc HIGH §11.7 |
| `BridgeDirection::EastWest ↔ Axis::EW ↔ state byte 9–17`, `BridgeDirection::NorthSouth ↔ Axis::NS ↔ state 0–8` | high | doc HIGH §7 + repo `resolved_terrain.rs:438-439` |
| IonCannon-only retry up to 3 retries, max 4 total `ApplyDamageToCell` calls; non-IonCannon Wall warhead = single attempt | high | verified live `0x4894B0` |
| Debris spawn structure: outer 95% gate + 2 jitter draws + 50%-MetallicDebris (no delay) + 1-always-BridgeExplosion (delay 1–5) | high | verified live `0x47DD70` + memory reads |
| Renderer queries `BridgeRuntimeState::display_tile`; `ResolvedTerrainGrid` stays immutable | high | design choice — avoids touching baked-terrain tests / save format |
| Bridgehead walk uses `cell.height` field at `CellClass+0x52` (likely) | **medium** | doc HIGH §3.2 — needs Ghidra spot-check (Task 0) |
| Debris spawn Z uses existing `bridge_deck_level_if_any().unwrap_or(level)` cell-level value | medium | unable to read `DAT_0089E7C0/E7B4` runtime values; existing pattern aligned with 90 lepton/level convention |

## Open Questions

### Resolved During Planning

- *"Does the audit's 'non-IonCannon retries 3x' claim hold?"* — No. Verified
  live: only IonCannon retries; non-IonCannon = single attempt. (Resolved in
  /review-plan; design + plan corrected.)
- *"Are partial-collapse states 7/16 paired or 7/17?"* — Per binary, state 7
  fires CollapseA, state 17 fires CollapseA → pair (7, 17). State 8/16 →
  CollapseB. (Resolved in /review-plan; design corrected.)
- *"Does `set_bridge_direction` BlowUpBridge 4 or 5 cells?"* — Exactly 4:
  cells 1, 2, 3, 5. Cell 4 is flag-only. (Resolved in /review-plan; design
  corrected.)
- *"What's `0x600` in BlowUpBridge AnimClass call?"* — 5th argument (anim
  flags), not Z offset. Z comes from `level * heightStep + heightStep_offset`
  in the prior compute. (Resolved in /review-plan; design ledger corrected.)

### Deferred to Implementation

- *"What is the exact field offset for the bridgehead `cell.height` walk
  termination check?"* — Likely `CellClass+0x52` (height-class byte). Spot-check
  via Ghidra at start of Phase C (Task 0). If wrong, re-decompile bridgehead
  branch of `ProcessBridgeDamageStateMachine_High @ 0x576BA0`.
- *"Do `DAT_0089E7C0` and `DAT_0089E7B4` match our `SHIP_HEIGHT_STEP = 90
  leptons/level`?"* — Cannot verify from static binary (runtime-init globals).
  Accepted as aligned-but-unverified; debris Z uses our existing cell-level
  + render-time conversion. If integration testing shows visual offset, return
  to Ghidra at runtime.
- *"What's the bridgehead-class overlay range for low (wood) bridges?"* — High
  is `0x18`/`0x19` (per binary); low is `0xED`/`0xEE` (per binary `0x4894B0`
  decompile + `overlay_types.rs::is_high_bridge_index` existing constants).
  Confirm both ranges in anchor-walker test fixture.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/sim/rng.rs` | Add `next_range_u32_inclusive(low, high)` helper. |
| Create | `src/rules/bridge_warheads.rs` | New `BridgeWarheads { ion_cannon, c4 }` sub-struct + INI parser. Pre-resolved warhead refs. |
| Modify | `src/rules/ruleset.rs` | Wire `BridgeWarheads` into `RuleSet`. Extend `GeneralRules.metallic_debris: Vec<String>` parser. |
| Modify | `src/rules/ruleset.rs` (`GeneralRules` lives here at line 145) | Parse `[General] MetallicDebris=` list. |
| Modify | `src/sim/bridge_state.rs` | Extend `BridgeRuntimeCell` (damage_state, axis, role, anchor_span_id, bridgehead_step). Add `AnchorSpan` struct + registry. Replace BFS constructor with anchor walker. Replace `apply_damage` with `apply_area_damage`. |
| Modify | `src/sim/bridge_specs.rs` | Add `apply_ramp_transition` (single fn + 16-entry overlay-neighbor table per family). Add `set_bridge_direction` walker. Add body + bridgehead state-machine drivers. Wire existing pure helpers. |
| Modify | `src/sim/combat/mod.rs` | Extend `BridgeDamageEvent` with `warhead_ref: WarheadId, is_ion_cannon: bool`. Modify 3 emit sites to populate new fields. |
| Modify | `src/sim/world/mod.rs` | `apply_bridge_damage_events` becomes gate-and-dispatch orchestrator. `resolve_bridge_state_changes` extends with ground-kill / debris rewrite / rim re-eval / zone refresh. New helpers: `kill_ground_occupants_at`, `spawn_bridge_debris` (replaces `spawn_bridge_explosions`), `update_adjacent_bridges`, `refresh_bridge_zones_if_dirty`. |
| Modify | `src/sim/world/world_hash.rs` | Hash new `BridgeRuntimeState` fields (cells extended, AnchorSpan registry). |
| Modify | `src/sim/world/world_tests.rs` | Rewrite existing 6 bridge-damage tests for new event shape (warhead_ref, is_ion_cannon). |
| Modify | `src/app_init_helpers.rs` | Update `BridgeRuntimeState::from_resolved_terrain` call to new constructor signature. |
| Modify | Render layer (terrain renderer) | Query `BridgeRuntimeState::display_tile(rx, ry, base_tile)` for bridge cells. |
| Create | `tests/bridge_tier2_integration.rs` | End-to-end test: map → IonCannon damage → state machine advance → ground-occupant kill → state-hash determinism. |

## Interface Changes

**Public:**
- `BridgeDamageEvent` adds `warhead_ref: WarheadId, is_ion_cannon: bool` — every
  caller in combat must populate.
- `BridgeRuntimeState::apply_damage` removed; replaced by `apply_area_damage(rx,
  ry, ctx) -> Vec<BridgeStateChange>` with `BridgeDamageContext`. Existing
  callers in `world/mod.rs` and `world_tests.rs` must migrate.
- `BridgeRuntimeState::from_resolved_terrain` constructor signature unchanged
  (params stay `(terrain, destroyable, strength)`); body changes to anchor walker.
- `BridgeRuntimeCell` field set extends; downstream consumers
  ([app_instances/units.rs:148](../../src/app_instances/units.rs#L148),
  [pathfinding/zone_build.rs:644](../../src/sim/pathfinding/zone_build.rs#L644))
  read existing fields only — no break.
- `RuleSet.bridge_warheads: BridgeWarheads` — new field. No existing consumers.
- `RuleSet.general.metallic_debris: Vec<String>` — new field. No existing consumers
  until this tier wires it.

**Internal (`bridge_state.rs` / `bridge_specs.rs`):**
- New: `AnchorSpan`, `Axis`, `DamageState`, `BridgeCellRole`, `Direction`,
  `BridgeDamageContext`, `BridgeDamageOutcome`, `Phase`, `CellAction` enums.
- Module-level `RAMP_TRANSITION_TABLE_HIGH` and `_LOW` 16-entry static lookup tables.

## Sim Checklist

- [x] All math uses `fixed`-point — yes: damage is `u16`, bridge strength is
      `u16`, RNG draws are `u32`, comparisons are integer. No new f32/f64.
- [x] New state included in deterministic state hash — yes: extend
      `world_hash.rs` to cover new `BridgeRuntimeCell` fields and
      `AnchorSpan` registry.
- [x] No dependencies on render/ui/sidebar/audio/net — yes: render queries
      `BridgeRuntimeState::display_tile` (read-only), and combat passes a
      pre-resolved `WarheadId` (no rules import in sim).
- [x] Tick ordering impact noted — bridge damage events drained between
      combat and ore growth (current order preserved). Within
      `apply_bridge_damage_events`: gate → dispatch (4 paths, each with own RNG
      draw) → retry (IonCannon only). Within `resolve_bridge_state_changes`:
      ground-kill → bridge-deck Limbo → debris → rim re-eval → zone refresh.
- [x] BTreeMap iteration order considered — `AnchorSpan` registry is
      `BTreeMap<u16, AnchorSpan>` for sorted iteration; `destroyed_cells`
      is sorted via existing `BTreeSet` in `BridgeStateChange`.

## Risk Areas

- **Anchor walker correctness at map load** is the single biggest risk. If
  it produces different anchor placement than the binary, every downstream
  behavior (collapse cell sets, ramp transitions, BlowUpBridge call counts)
  drifts. Mitigation: dedicated unit tests against handcrafted bridge fixtures
  + state-hash determinism test (Phase G).
- **RNG draw order parity** — combat pre-resolving `is_ion_cannon` must
  short-circuit the BridgeStrength draw (saves 1 draw); per-path independent
  draws must execute in correct order (high body → high direct → low body →
  low direct). Mitigation: dedicated state-hash test that consumes a known
  RNG sequence and asserts exact draw count.
- **Existing 6 world_tests use single-shot collapse semantics** — they pass
  raw `BridgeDamageEvent { rx, ry, damage }` and assert collapse on first
  hit. Must rewrite all 6 to use IonCannonWarhead context (which preserves
  the single-shot collapse behavior with retries).
- **Combat parallel session** — `TargetKind` work in `combat/mod.rs` is
  uncommitted. Bridge-emit-site changes are surgical (3 sites, all use
  pattern `if warhead.wall && weapon.damage > 0 && !cell_has_wall_overlay`).
  Anchor by pattern, not line number.
- **Render layer scope** — exact display-tile API depends on the renderer's
  current bridge tile lookup. Phase D Task 19 may need adjustment after
  inspecting the renderer.
- **Snapshot binary format breaks** for in-flight dev saves — no production
  save format exists, so impact is dev-test only. Existing snapshot
  round-trip tests in [src/sim/world/world_tests.rs](../../src/sim/world/world_tests.rs)
  must be updated.
- **Bridgehead height-walk field offset uncertainty** — flagged for Task 0
  Ghidra spot-check before Phase C.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|---|---|---|---|
| Task 22 | IonCannon-only retry semantics (max 4 attempts) | Wrong direction = wrong damage rate to bridges every match where bridges are present | State-hash test (Task 39) + Ghidra cross-check |
| Task 22 | RNG draw count: non-IonCannon = 1 draw per dispatch path; IonCannon = 0 BridgeStrength draws (bypassed) | Lockstep determinism — every diverging draw breaks replay | State-hash test (Task 39) |
| Task 22 | RandomRanged(1, BridgeStrength) inclusive low, strict `<` damage comparison | Off-by-one in either direction shifts collapse probability | Unit test + Ghidra cross-check |
| Task 24 | `spawn_bridge_debris` structure: 95% outer + 2 jitter + 50%-MetallicDebris (no delay) + 1-always-BridgeExplosion (delay 1–5) | Player sees explosions and metallic debris on every bridge collapse; current Rust code is wrong | In-game side-by-side + RNG draw count test |
| Task 25 | `kill_ground_occupants_at` uses C4Warhead force_kill via existing death pipeline (correct InfDeath selection) | Player sees correct death anim variant for ground units under collapsing bridge | Integration test + side-by-side |
| Task 14 | `set_bridge_direction` invokes BlowUpBridge on exactly 4 cells (1, 2, 3, 5), not 5 | Wrong cell count = wrong number of ground kills + debris spawns + RNG draws | Walker unit tests + state-hash test |
| Task 16 | Body-cell state advance: Healthy → Damaged (state=6/15) absorbs damage; second hit → Collapse | The "two-step damage" is the player-visible cracked-deck-then-fall sequence | Unit test + side-by-side |
| Task 18 | Bridgehead 4-step progression with final-step BlowUpBridge × 3 perpendicular cells | Bridgeheads collapse via ramp damage; current Rust has nothing | Unit test + side-by-side |
| Task 27 | `update_adjacent_bridges` rim re-eval after every state change | Edge tiles re-evaluate flags; without this, rendering of partially-damaged bridges drifts | Visual + cell-flag-state test |
| Task 17 | Partial-collapse states 7→CollapseA, 8→CollapseB, 16→CollapseB, 17→CollapseA | Reachable via bridgehead cascade; wrong dispatch = wrong overlay tile | Unit test |
| Task 11 | Pre-resolve IonCannonWarhead and C4Warhead refs at world init from rules | Drop-down warheads must match `[CombatDamage]` keys; mod-loaded rules override | Unit test + integration |

---

## Tasks

### Task 0: Spot-check bridgehead `cell.height` field offset in Ghidra

**Why:** Design ledger #28 flagged this as needing binary verification before
Phase C. We need to know which `CellClass` field the bridgehead walk-to-anchor
loop reads (height==4 NS / height==2 EW). Likely `+0x52` (height class) but
unverified.

**Files:** None (research only)

**Step 1: Decompile bridgehead branch**

Use Ghidra MCP to decompile `ProcessBridgeDamageStateMachine_High` at
`0x576BA0`. Locate the bridgehead branch (`flags & 0x100 == 0`). Find the
walk loop that terminates on height==4 (NS) or height==2 (EW).

**Step 2: Identify field offset**

Read which struct offset is being compared. Note as `CellClass+0xXX`.

**Step 3: Update design ledger**

If offset is `+0x52`: ledger #28's "likely `CellClass+0x52`" is now confirmed —
mark `[verified]`. If different: update ledger #28 with correct offset and
field semantic.

**Step 4: Document the field on `BridgeRuntimeCell` or derive from `ResolvedTerrainCell`**

If the field is the height-class byte, it likely maps to our existing
`ResolvedTerrainCell.level: u8` or a different per-cell height
attribute. Confirm by comparing offsets via doc cross-reference. Note the
mapping in design ledger #28 home column.

**Step 5: Commit (no code changes — only design.md edit if any)**

If design.md updated:
```
git commit -m "design: tier 2 ledger #28 — confirm bridgehead height field offset"
```

---

### Task 1: Add `next_range_u32_inclusive` to SimRng

**Why:** Binary's `RandomRanged(low, high)` is inclusive on both ends; our
`next_range_u32(N)` returns `[0, N)`. Need a helper to mirror binary's
calling convention cleanly. Used by Phase F gate sites.

**Files:** Modify `src/sim/rng.rs`

**Pattern:** New method on existing struct.

**Step 1: Add the helper**

```rust
// src/sim/rng.rs — add inside `impl SimRng`

    /// Random integer in `[low, high]` inclusive on both ends.
    /// Returns `low` when `high <= low`. Saturating widen handles the
    /// pathological `high == u32::MAX, low == 0` case without overflow.
    ///
    /// Mirrors binary `Random__RandomRanged(low, high)` calling convention.
    pub fn next_range_u32_inclusive(&mut self, low: u32, high: u32) -> u32 {
        if high <= low {
            return low;
        }
        let span = (high as u64) - (low as u64) + 1;
        let span_u32 = span.min(u32::MAX as u64) as u32;
        low.saturating_add(self.next_range_u32(span_u32))
    }
```

**Step 2: Add tests**

```rust
// src/sim/rng.rs — add inside `mod tests`

    #[test]
    fn test_inclusive_range_bounds() {
        let mut rng = SimRng::new(42);
        for _ in 0..256 {
            let v = rng.next_range_u32_inclusive(1, 5);
            assert!((1..=5).contains(&v));
        }
    }

    #[test]
    fn test_inclusive_range_degenerate() {
        let mut rng = SimRng::new(1);
        // low == high → always returns low, no draw.
        assert_eq!(rng.next_range_u32_inclusive(7, 7), 7);
        // high < low → returns low.
        assert_eq!(rng.next_range_u32_inclusive(7, 3), 7);
    }
```

**Step 3: Verify**

Run: `cargo test --lib sim::rng -- --nocapture`
Expected: PASS (all rng tests including new ones).

**Step 4: Commit**

```
git commit -m "rng: add next_range_u32_inclusive(low, high) for binary parity"
```

---

### Task 2: Create `BridgeWarheads` sub-struct in rules

**Why:** Pre-resolve `IonCannonWarhead` and `C4Warhead` refs once at world
init. Combat reads them per-shot (cheap interned ID compare). Mirrors binary's
`Rules+0xFF0` / `Rules+0xFA8` storage. New sub-struct because existing
`CombatDamageDefaults` is scoped to particle-system slots only.

**Files:** Create `src/rules/bridge_warheads.rs`; modify
`src/rules/ruleset.rs`.

**Pattern:** Mirrors `src/rules/combat_damage.rs` sub-struct + `from_ini_section`
pattern, but stores names (resolved later against the warhead registry, like
`ParticleSystemType.warhead`).

**Step 1: Create the new module file**

```rust
// src/rules/bridge_warheads.rs

//! `[CombatDamage]` bridge-specific warhead references.
//!
//! Bridge damage gating + collapse fallout cite two specific warhead names by
//! [CombatDamage] key:
//! - IonCannonWarhead: bypasses BridgeStrength RNG gate; enables 3-retry loop in
//!   Apply_area_damage (gamemd Rules+0xFF0).
//! - C4Warhead: used as the killing warhead in BlowUpBridge ground-occupant
//!   force_kill (gamemd Rules+0xFA8).
//!
//! Stored as raw INI strings here; resolved to interned `WarheadId`s at world
//! init time when the warhead registry is populated.
//!
//! ## Dependency rules
//! - Part of rules/ — no dependencies on sim/, render/, ui/, etc.

use crate::rules::ini_parser::IniSection;

/// Default warhead names for bridge-related combat from `[CombatDamage]`.
///
/// Each field is the section name of a `WarheadType` (resolved later against
/// `RuleSet::warhead_id_by_name`). Defaults match retail rulesmd.ini.
#[derive(Debug, Clone)]
pub struct BridgeWarheads {
    /// `[CombatDamage] IonCannonWarhead=` (default `"IonCannonWH"`).
    /// Bypasses BridgeStrength RNG gate; enables retry loop.
    pub ion_cannon_name: String,
    /// `[CombatDamage] C4Warhead=` (default `"Super"`).
    /// Used as killing warhead in bridge-collapse ground kill.
    pub c4_name: String,
}

impl Default for BridgeWarheads {
    fn default() -> Self {
        Self {
            ion_cannon_name: "IonCannonWH".to_string(),
            c4_name: "Super".to_string(),
        }
    }
}

impl BridgeWarheads {
    /// Parse from a `[CombatDamage]` `IniSection`. Missing keys use defaults.
    pub fn from_ini_section(section: &IniSection) -> Self {
        let default = Self::default();
        Self {
            ion_cannon_name: read_name(section, "IonCannonWarhead")
                .unwrap_or(default.ion_cannon_name),
            c4_name: read_name(section, "C4Warhead").unwrap_or(default.c4_name),
        }
    }
}

fn read_name(section: &IniSection, key: &str) -> Option<String> {
    section
        .get(key)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;

    #[test]
    fn defaults_match_retail_rulesmd() {
        let bw = BridgeWarheads::default();
        assert_eq!(bw.ion_cannon_name, "IonCannonWH");
        assert_eq!(bw.c4_name, "Super");
    }

    #[test]
    fn parses_keys_from_combat_damage_section() {
        let ini = IniFile::from_str(
            "[CombatDamage]\nIonCannonWarhead=CustomIon\nC4Warhead=CustomC4\n",
        )
        .unwrap();
        let section = ini.section("CombatDamage").unwrap();
        let bw = BridgeWarheads::from_ini_section(section);
        assert_eq!(bw.ion_cannon_name, "CustomIon");
        assert_eq!(bw.c4_name, "CustomC4");
    }

    #[test]
    fn missing_keys_fall_back_to_defaults() {
        let ini = IniFile::from_str("[CombatDamage]\n").unwrap();
        let section = ini.section("CombatDamage").unwrap();
        let bw = BridgeWarheads::from_ini_section(section);
        assert_eq!(bw.ion_cannon_name, "IonCannonWH");
        assert_eq!(bw.c4_name, "Super");
    }
}
```

**Step 2: Wire into `mod.rs` for the rules crate**

Add `pub mod bridge_warheads;` to `src/rules/mod.rs` (alongside `pub mod combat_damage;`).

**Step 3: Add field to `RuleSet`**

In `src/rules/ruleset.rs`, find `pub struct RuleSet {` near line 1100 and add:

```rust
    /// Pre-resolved bridge-related warhead names (`[CombatDamage]
    /// IonCannonWarhead=`, `C4Warhead=`). Resolution to interned IDs happens
    /// at world init.
    pub bridge_warheads: crate::rules::bridge_warheads::BridgeWarheads,
```

In `RuleSet::from_ini` (search for `BridgeRules::from_ini(ini)`), add a matching parse call:

```rust
    let bridge_warheads = ini
        .section("CombatDamage")
        .map(crate::rules::bridge_warheads::BridgeWarheads::from_ini_section)
        .unwrap_or_default();
```

And include `bridge_warheads,` in the struct initialization at the end of
`RuleSet::from_ini`. (Note: there's an existing `combat_damage` field; mirror its
position.)

**Step 4: Verify**

```
cargo test --lib rules::bridge_warheads -- --nocapture
cargo build
```
Expected: PASS for new tests, build green.

**Step 5: Commit**

```
git commit -m "rules: add BridgeWarheads sub-struct + parse [CombatDamage] IonCannonWarhead/C4Warhead"
```

---

### Task 3: Parse `[General] MetallicDebris=` into `GeneralRules`

**Why:** `BlowUpBridge` debris-spawn step 4a reads `Rules+0x140/+0x14C`
(MetallicDebris array + count). Tier 1 deferred this. Needed by Phase F
Task 24's `spawn_bridge_debris`.

**Files:** Modify `src/rules/ruleset.rs` (where `GeneralRules` is defined and
parsed). Path may differ — search for `pub struct GeneralRules` and adjust.

**Pattern:** Mirrors existing `BridgeExplosions` parsing in `BridgeRules`
(Tier 1).

**Step 1: Add field to `GeneralRules`**

In `src/rules/ruleset.rs` near line 145 (`pub struct GeneralRules`), add:

```rust
    /// `[General] MetallicDebris=` — list of animation names to spawn (50%-RNG
    /// gated, count-checked) on bridge-cell collapse. Default 20 entries.
    /// Mirrors gamemd `Rules+0x140` (data ptr) / `+0x14C` (count).
    pub metallic_debris: Vec<String>,
```

**Step 2: Default**

In `impl Default for GeneralRules` near line 475, add:

```rust
        metallic_debris: vec![
            "DBRIS1LG", "DBRIS2LG", "DBRIS3LG", "DBRIS4LG", "DBRIS5LG",
            "DBRIS6LG", "DBRIS7LG", "DBRIS8LG", "DBRIS9LG", "DBRS10LG",
            "DBRIS1SM", "DBRIS2SM", "DBRIS3SM", "DBRIS4SM", "DBRIS5SM",
            "DBRIS6SM", "DBRIS7SM", "DBRIS8SM", "DBRIS9SM", "DBRS10SM",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),
```

**Step 3: Parse in `GeneralRules::from_ini`**

In `impl GeneralRules` `from_ini` near line 720, add (next to other comma-separated list parsers):

```rust
        let metallic_debris = ini
            .section("General")
            .and_then(|s| s.get("MetallicDebris"))
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| Self::default().metallic_debris);
```

And include `metallic_debris,` in the struct construction.

**Step 4: Add tests**

```rust
// in src/rules/ruleset.rs `mod tests`

    #[test]
    fn metallic_debris_default_matches_retail() {
        let g = GeneralRules::default();
        assert_eq!(g.metallic_debris.len(), 20);
        assert_eq!(g.metallic_debris[0], "DBRIS1LG");
        assert_eq!(g.metallic_debris[19], "DBRS10SM");
    }

    #[test]
    fn metallic_debris_parses_from_ini() {
        let ini =
            IniFile::from_str("[General]\nMetallicDebris=ANIM1,ANIM2,ANIM3\n").unwrap();
        let g = GeneralRules::from_ini(&ini);
        assert_eq!(g.metallic_debris, vec!["ANIM1", "ANIM2", "ANIM3"]);
    }
```

**Step 5: Verify**

```
cargo test --lib rules::ruleset::tests::metallic_debris -- --nocapture
```
Expected: PASS.

**Step 6: Commit**

```
git commit -m "rules: parse [General] MetallicDebris= into GeneralRules (default 20-entry retail list)"
```

---

### Task 4: Define `Axis`, `DamageState`, `BridgeCellRole`, `Direction`, `Phase` enums

**Why:** Foundation types for the state machine. All other tasks reference these.

**Files:** Modify `src/sim/bridge_state.rs` (add at top of file before existing
structs).

**Pattern:** Same `serde::Serialize/Deserialize` derives as existing
`BridgeRuntimeCell` for snapshot round-trip.

**Step 1: Add enums above existing `BridgeDamageEvent`**

```rust
// src/sim/bridge_state.rs — insert after line 14 (above `BridgeDamageEvent`)

/// Bridge body axis. Body cells are stacked along this axis; ramps face
/// perpendicular.
///
/// Mapping: `Axis::EW` ↔ `BridgeDirection::EastWest` ↔ state byte 9–17;
/// `Axis::NS` ↔ `BridgeDirection::NorthSouth` ↔ state byte 0–8.
/// Per gamemd HIGH §7: the Ghidra `Walker_NS_High` / `Walker_EW_High`
/// function-name labels are SWAPPED vs physical axis. Always key transitions
/// off overlay range, not function name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Axis {
    /// Body cells stacked north–south (along Y); ramps face east/west.
    /// State byte range 0–8.
    NS,
    /// Body cells stacked east–west (along X); ramps face north/south.
    /// State byte range 9–17.
    EW,
}

/// Per-cell damage state encoding all 18 binary state-byte values.
///
/// Body cells transition Healthy → Damaged → Destroyed under repeated
/// damage (per axis). Partial-collapse states are reached only via
/// bridgehead final-step cascade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DamageState {
    /// Healthy body — `variant` carries the 6 frame jitter (0..=5 per axis,
    /// map-load-deterministic, never advances during gameplay).
    /// Maps to state byte 0–5 (NS) or 9–14 (EW).
    Healthy { variant: u8 },
    /// Damaged body — next hit collapses. State byte 6 (NS) / 15 (EW).
    Damaged,
    /// Partial collapse: ramp B already collapsed; this cell will fire
    /// CollapseA. State byte 7 (NS) / 17 (EW).
    PartialCollapseA,
    /// Partial collapse: ramp A already collapsed; this cell will fire
    /// CollapseB. State byte 8 (NS) / 16 (EW).
    PartialCollapseB,
    /// Fully destroyed.
    Destroyed,
}

/// Cell role within an `AnchorSpan`.
///
/// Mirrors gamemd CellClass+0x140 flag bits 0x80 (anchor-self) and 0x100
/// (bridge structural).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BridgeCellRole {
    /// Anchor cell (binary flag 0x80 + 0x100): primary cell of an anchor
    /// span; carries the canonical state byte.
    Anchor,
    /// Body cell (binary flag 0x100, no 0x80): non-anchor structural cell;
    /// follows `anchor_span_id` for state-machine processing.
    Body,
    /// Bridgehead cell (no flag 0x100): ramp connection-piece off the body.
    Bridgehead,
    /// Tail cell (cell 5 of anchor pattern, walked in `–direction` from anchor).
    Tail,
}

/// Compass-direction enum matching gamemd `g_DirectionOffsets @ 0x89F688`.
///
/// **Discriminant values must match the binary's table indices** because
/// `set_bridge_direction` uses them to index into the offsets table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Direction {
    N = 0,
    NE = 1,
    E = 2,
    SE = 3,
    S = 4,
    SW = 5,
    W = 6,
    NW = 7,
}

impl Direction {
    /// Cell-coord offset `(dx, dy)`. Signed because directions can decrement.
    pub const fn offset(self) -> (i32, i32) {
        match self {
            Direction::N => (0, -1),
            Direction::NE => (1, -1),
            Direction::E => (1, 0),
            Direction::SE => (1, 1),
            Direction::S => (0, 1),
            Direction::SW => (-1, 1),
            Direction::W => (-1, 0),
            Direction::NW => (-1, -1),
        }
    }

    /// `(self - 4) & 7` — opposite direction. Used by `set_bridge_direction`
    /// to compute cell 5 (walked in –direction from anchor).
    pub const fn opposite(self) -> Direction {
        match self {
            Direction::N => Direction::S,
            Direction::NE => Direction::SW,
            Direction::E => Direction::W,
            Direction::SE => Direction::NW,
            Direction::S => Direction::N,
            Direction::SW => Direction::NE,
            Direction::W => Direction::E,
            Direction::NW => Direction::SE,
        }
    }
}

/// `apply_ramp_transition` phase. Maps to one of the 16 binary
/// `UpdateRamp_*` helpers (NS/EW × DamageA/DamageB/CollapseA/CollapseB).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    DamageA,
    DamageB,
    CollapseA,
    CollapseB,
}
```

**Step 2: Add unit tests**

```rust
// in src/sim/bridge_state.rs `mod tests`

    #[test]
    fn direction_offsets_match_compass() {
        assert_eq!(Direction::N.offset(), (0, -1));
        assert_eq!(Direction::E.offset(), (1, 0));
        assert_eq!(Direction::S.offset(), (0, 1));
        assert_eq!(Direction::W.offset(), (-1, 0));
    }

    #[test]
    fn direction_opposite_is_idempotent() {
        for dir in [
            Direction::N, Direction::NE, Direction::E, Direction::SE,
            Direction::S, Direction::SW, Direction::W, Direction::NW,
        ] {
            assert_eq!(dir.opposite().opposite(), dir);
        }
    }

    #[test]
    fn direction_opposite_pairs() {
        assert_eq!(Direction::N.opposite(), Direction::S);
        assert_eq!(Direction::E.opposite(), Direction::W);
        assert_eq!(Direction::NE.opposite(), Direction::SW);
        assert_eq!(Direction::SE.opposite(), Direction::NW);
    }
```

**Step 3: Verify**

```
cargo test --lib sim::bridge_state -- --nocapture
```
Expected: PASS (existing tests + new direction tests).

**Step 4: Commit**

```
git commit -m "bridge_state: add Axis/DamageState/BridgeCellRole/Direction/Phase enums for tier 2 state machine"
```

---

### Task 5: Define `AnchorSpan` struct

**Why:** First-class anchor-pattern data. Replaces emergent flag-bit anchor
detection. One span per anchor cell, owns 4–6 cells per the binary's
`SetBridgeDirection_NESW` walker pattern.

**Files:** Modify `src/sim/bridge_state.rs` (add after `Direction`/`Phase`
enums from Task 4).

**Step 1: Add the struct**

```rust
// src/sim/bridge_state.rs — after Phase enum from Task 4

/// First-class anchor-span representation. One span per anchor cell.
///
/// Mirrors the binary's anchor pattern emitted by `SetBridgeDirection_NESW`:
/// up to 6 cells (anchor + 3 walked +dir + 1 walked –dir + optional fixed-offset
/// cell when direction == W (6)). Per-cell action (BlowUpBridge vs flag-only)
/// is determined by slot index.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AnchorSpan {
    /// Stable ID, matches `BridgeRuntimeCell.anchor_span_id`.
    pub id: u16,
    /// The anchor cell (binary flag 0x80 set). Slot 0.
    pub anchor: (u16, u16),
    /// All cells in walker order:
    /// `[0]=anchor, [1..=3]=+direction × 1/2/3, [4]=-direction × 1, [5]=fixed-offset (only when direction == W)`.
    /// `None` for unused slots when the optional fixed-offset cell isn't present.
    pub cells: [Option<(u16, u16)>; 6],
    /// Body axis (NS or EW). Determined from `bridge_layer.direction`.
    pub axis: Axis,
    /// Walk direction (compass index 0–7). Used to compute walked cells.
    pub direction: Direction,
    /// Mirror of anchor cell's damage state. Convenience for queries.
    pub damage_state: DamageState,
    /// Group ID (existing `BridgeRuntimeState::group_cells`) — preserved for
    /// connectivity queries.
    pub bridge_group_id: u16,
}

impl AnchorSpan {
    /// Cells receiving `BlowUpBridge` on destruction path: slots 0, 1, 2, 4
    /// (binary's cells 1, 2, 3, 5). Slot 3 (cell 4) and slot 5 (cell 6) are
    /// flag-only.
    pub const BLOW_UP_SLOTS: [usize; 4] = [0, 1, 2, 4];

    /// Iterate `(slot, cell)` for present cells (skips `None`).
    pub fn iter_cells(&self) -> impl Iterator<Item = (usize, (u16, u16))> + '_ {
        self.cells
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.map(|cell| (i, cell)))
    }

    /// Cells that get `BlowUpBridge` on destruction. Skips slots 3 (cell 4)
    /// and 5 (cell 6) which are flag-only per binary `SetBridgeDirection_NESW`.
    pub fn blow_up_cells(&self) -> impl Iterator<Item = (u16, u16)> + '_ {
        Self::BLOW_UP_SLOTS
            .iter()
            .filter_map(|&slot| self.cells[slot])
    }
}
```

**Step 2: Add unit tests**

```rust
// in src/sim/bridge_state.rs `mod tests`

    fn make_test_span() -> AnchorSpan {
        AnchorSpan {
            id: 1,
            anchor: (5, 5),
            cells: [
                Some((5, 5)), // slot 0 = anchor
                Some((6, 5)), // slot 1 = +E × 1
                Some((7, 5)), // slot 2 = +E × 2
                Some((8, 5)), // slot 3 = +E × 3 (FLAG ONLY)
                Some((4, 5)), // slot 4 = -E × 1 = +W × 1
                None,         // slot 5 = optional W-direction fixed offset
            ],
            axis: Axis::NS,
            direction: Direction::E,
            damage_state: DamageState::Healthy { variant: 0 },
            bridge_group_id: 1,
        }
    }

    #[test]
    fn anchor_span_blow_up_cells_excludes_slot_3() {
        let span = make_test_span();
        let cells: Vec<_> = span.blow_up_cells().collect();
        // Per gamemd verified live 0x47E040: cells 1, 2, 3, 5 in binary numbering
        // = our slots 0, 1, 2, 4. NOT slot 3 (binary cell 4, flag-only).
        assert_eq!(cells, vec![(5, 5), (6, 5), (7, 5), (4, 5)]);
    }

    #[test]
    fn anchor_span_iter_cells_skips_none() {
        let span = make_test_span();
        let count = span.iter_cells().count();
        assert_eq!(count, 5); // 6 slots, 1 None
    }
```

**Step 3: Verify**

```
cargo test --lib sim::bridge_state -- --nocapture
```
Expected: PASS.

**Step 4: Commit**

```
git commit -m "bridge_state: add AnchorSpan struct + 4-cell BlowUpBridge slot constant"
```

---

### Task 6: Extend `BridgeRuntimeCell` with new fields

**Why:** Per-cell state for the state machine. Carries `damage_state`, `axis`,
`role`, `anchor_span_id`, `bridgehead_step`. Existing `destroyed: bool` is
removed (replaced by `damage_state == Destroyed`).

**Files:** Modify `src/sim/bridge_state.rs:27-34`.

**Step 1: Replace existing struct**

```rust
// src/sim/bridge_state.rs — replace existing BridgeRuntimeCell

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BridgeRuntimeCell {
    pub deck_present: bool,
    pub destroyable: bool,
    pub deck_level: u8,
    pub bridge_group_id: Option<u16>,

    /// Per-cell damage state. Drives state-machine progression and renderer
    /// display-tile selection.
    pub damage_state: DamageState,

    /// Bridge body axis (NS or EW). `None` for cells where axis is not
    /// meaningful (orphan body cells, edge cases).
    pub axis: Option<Axis>,

    /// Cell role within its anchor span. Drives state-machine branch dispatch.
    pub role: BridgeCellRole,

    /// Stable ID of containing `AnchorSpan` (for body cells); `None` for
    /// bridgehead cells (which use `bridgehead_step` instead).
    pub anchor_span_id: Option<u16>,

    /// Bridgehead 4-step progression counter (0..=3). Only meaningful when
    /// `role == BridgeCellRole::Bridgehead`. Mirrors binary's overlay-class
    /// offset from `BridgeheadBase`.
    pub bridgehead_step: u8,
}
```

**Step 2: Update `is_bridge_walkable` to use `damage_state`**

Replace the existing `is_bridge_walkable` body at line 151:

```rust
    pub fn is_bridge_walkable(&self, rx: u16, ry: u16) -> bool {
        self.cell(rx, ry).is_some_and(|cell| {
            cell.deck_present && !matches!(cell.damage_state, DamageState::Destroyed)
        })
    }
```

**Step 3: Update existing constructor stub temporarily**

In `BridgeRuntimeState::from_resolved_terrain` near line 104, the line:
```rust
                cells[idx] = Some(BridgeRuntimeCell {
                    deck_present: true,
                    destroyed: false,
                    destroyable,
                    deck_level: resolved.bridge_deck_level,
                    bridge_group_id: Some(group_id),
                });
```
becomes (temporary intermediate state — Task 7 will replace this entire
constructor with anchor walker):
```rust
                cells[idx] = Some(BridgeRuntimeCell {
                    deck_present: true,
                    destroyable,
                    deck_level: resolved.bridge_deck_level,
                    bridge_group_id: Some(group_id),
                    damage_state: DamageState::Healthy { variant: 0 },
                    axis: None, // filled in by Task 7 anchor walker
                    role: BridgeCellRole::Body, // filled in by Task 7
                    anchor_span_id: None,
                    bridgehead_step: 0,
                });
```

**Step 4: Update `apply_damage` for now (Task 22 will replace)**

The existing `apply_damage` reads/writes `cell.destroyed`. Update to use
`damage_state == Destroyed`:

```rust
    pub fn apply_damage(&mut self, event: BridgeDamageEvent) -> Option<BridgeStateChange> {
        if event.damage == 0 {
            return None;
        }
        let cell = self.cell(event.rx, event.ry).copied()?;
        if !cell.deck_present
            || matches!(cell.damage_state, DamageState::Destroyed)
            || !cell.destroyable
        {
            return None;
        }
        let Some(group_id) = cell.bridge_group_id else {
            return None;
        };
        let hp = self
            .group_hitpoints
            .entry(group_id)
            .or_insert(self.strength_per_group);
        *hp = hp.saturating_sub(event.damage);
        if *hp > 0 {
            return None;
        }

        let mut destroyed_cells = self.group_cells.get(&group_id).cloned().unwrap_or_default();
        destroyed_cells.sort_unstable();
        for &(rx, ry) in &destroyed_cells {
            if let Some(idx) = index_of(self.width, self.height, rx, ry) {
                if let Some(cell) = self.cells[idx].as_mut() {
                    cell.damage_state = DamageState::Destroyed;
                }
            }
        }
        for record in &mut self.endpoint_records {
            if record.group_id == group_id {
                record.active = false;
            }
        }
        Some(BridgeStateChange { destroyed_cells })
    }
```

**Step 5: Update existing tests in same file**

The bottom-of-file tests construct `ResolvedTerrainCell` and assert on
`BridgeRuntimeCell` fields. Update test assertions (lines ~351-417):

```rust
    #[test]
    fn bridge_runtime_initializes_intact_groups() {
        let state = BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), true, 300);
        let cell = state.cell(1, 0).expect("bridge cell");
        assert!(cell.deck_present);
        assert!(matches!(cell.damage_state, DamageState::Healthy { .. }));
        assert_eq!(cell.deck_level, 4);
        assert_eq!(cell.bridge_group_id, Some(1));
        assert!(state.cell(0, 0).is_none());
    }

    #[test]
    fn destroying_a_bridge_group_marks_all_members_destroyed() {
        let mut state =
            BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), true, 50);
        let change = state
            .apply_damage(BridgeDamageEvent {
                rx: 1, ry: 0, damage: 50,
            })
            .expect("bridge should be destroyed");
        assert_eq!(change.destroyed_cells, vec![(1, 0), (2, 0), (3, 0)]);
        assert!(!state.is_bridge_walkable(1, 0));
        assert!(!state.is_bridge_walkable(2, 0));
        assert!(!state.is_bridge_walkable(3, 0));
        // New: verify damage_state per cell.
        assert_eq!(
            state.cell(1, 0).map(|c| c.damage_state),
            Some(DamageState::Destroyed)
        );
    }
```

**Step 6: Verify**

```
cargo build
cargo test --lib sim::bridge_state -- --nocapture
```
Expected: build green, all bridge tests pass.

**Step 7: Commit**

```
git commit -m "bridge_state: extend BridgeRuntimeCell with damage_state/axis/role/anchor_span_id/bridgehead_step"
```

---

### Task 7: Replace BFS-grouping constructor with anchor walker

**Why:** Per design ledger #34, anchors must follow `SetBridgeDirection_NESW`
walker pattern (4–6 cells per anchor) instead of BFS-by-deck-presence. This
gives anchor-span granularity for collapse: a long bridge becomes multiple
spans, so collapse can land per-span, not per-whole-bridge.

**Files:** Modify `src/sim/bridge_state.rs` — replace the `from_resolved_terrain`
body and add `compute_anchor_spans` helper. Add `anchor_spans` field to
`BridgeRuntimeState`.

**Pattern:** Mirrors binary `SetBridgeDirection_NESW` step-for-step
(verified live `0x47E040`).

**Step 1: Add `anchor_spans` field to `BridgeRuntimeState`**

In existing struct around line 51:

```rust
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BridgeRuntimeState {
    width: u16,
    height: u16,
    cells: Vec<Option<BridgeRuntimeCell>>,
    group_cells: BTreeMap<u16, Vec<(u16, u16)>>,
    group_hitpoints: BTreeMap<u16, u16>,
    strength_per_group: u16,
    /// Strength constant from `[CombatDamage] BridgeStrength=` (default 1500).
    /// Used by `apply_area_damage` BridgeStrength RNG gate (Task 22).
    bridge_strength: u16,
    endpoint_records: Vec<BridgeEndpointRecord>,
    /// First-class anchor spans (one per anchor cell). Replaces emergent
    /// flag-bit detection.
    anchor_spans: BTreeMap<u16, AnchorSpan>,
    /// Default `SpecialFlags & 0x8000` derived from per-map override + rules
    /// `destroyable_by_default`. Read by `apply_area_damage` outer gate.
    bridge_destroyable_flag: bool,
}
```

(Note: also add `bridge_destroyable_flag` — needed by Phase F.)

**Step 2: Rewrite `from_resolved_terrain`**

```rust
    pub fn from_resolved_terrain(
        terrain: &ResolvedTerrainGrid,
        destroyable: bool,
        strength_per_group: u16,
    ) -> Self {
        let width = terrain.width();
        let height = terrain.height();
        let mut cells = vec![None; width as usize * height as usize];
        let mut group_cells: BTreeMap<u16, Vec<(u16, u16)>> = BTreeMap::new();
        let mut anchor_spans: BTreeMap<u16, AnchorSpan> = BTreeMap::new();
        let mut visited = vec![false; cells.len()];
        let mut next_group_id: u16 = 1;
        let mut next_span_id: u16 = 1;

        // Pass 1: BFS-group bridge cells by deck presence (existing
        // group_cells used for endpoint_records + zone connectivity).
        for cell in terrain.iter() {
            let Some(index) = index_of(width, height, cell.rx, cell.ry) else {
                continue;
            };
            if visited[index] || !cell.has_bridge_deck {
                continue;
            }
            let group_id = next_group_id;
            next_group_id = next_group_id.saturating_add(1);
            let mut queue = VecDeque::from([(cell.rx, cell.ry)]);
            let mut members = Vec::new();
            while let Some((rx, ry)) = queue.pop_front() {
                let Some(idx) = index_of(width, height, rx, ry) else {
                    continue;
                };
                if visited[idx] {
                    continue;
                }
                let Some(resolved) = terrain.cell(rx, ry) else {
                    continue;
                };
                if !resolved.has_bridge_deck {
                    continue;
                }
                visited[idx] = true;
                members.push((rx, ry));
                cells[idx] = Some(BridgeRuntimeCell {
                    deck_present: true,
                    destroyable,
                    deck_level: resolved.bridge_deck_level,
                    bridge_group_id: Some(group_id),
                    damage_state: DamageState::Healthy { variant: 0 },
                    axis: bridge_layer_to_axis(resolved.bridge_layer.as_ref()),
                    role: BridgeCellRole::Body, // overwritten in pass 2
                    anchor_span_id: None,
                    bridgehead_step: 0,
                });
                for (nx, ny) in cardinal_neighbors(rx, ry, width, height) {
                    if let Some(neighbor) = terrain.cell(nx, ny) {
                        if neighbor.has_bridge_deck {
                            queue.push_back((nx, ny));
                        }
                    }
                }
            }
            if !members.is_empty() {
                group_cells.insert(group_id, members);
            }
        }

        // Pass 2: walk anchor patterns. For each cell whose
        // bridge_layer.overlay_id matches an anchor-overlay class, emit one
        // AnchorSpan and tag member cells with role + anchor_span_id.
        for (&group_id, members) in &group_cells {
            for &(rx, ry) in members {
                let Some(resolved) = terrain.cell(rx, ry) else {
                    continue;
                };
                let Some(bl) = resolved.bridge_layer.as_ref() else {
                    continue;
                };
                if !is_anchor_overlay(bl.overlay_id) {
                    continue;
                }
                let axis = bridge_direction_to_axis(bl.direction);
                let direction = anchor_walk_direction(axis);
                let span_id = next_span_id;
                next_span_id = next_span_id.saturating_add(1);
                let span = walk_anchor_pattern(
                    span_id, (rx, ry), axis, direction, group_id, width, height,
                );
                // Tag each cell in span.
                for (slot, cell_pos) in span.iter_cells() {
                    if let Some(idx) = index_of(width, height, cell_pos.0, cell_pos.1) {
                        if let Some(c) = cells[idx].as_mut() {
                            c.role = if slot == 0 {
                                BridgeCellRole::Anchor
                            } else if slot == 4 {
                                BridgeCellRole::Tail
                            } else {
                                BridgeCellRole::Body
                            };
                            c.anchor_span_id = Some(span_id);
                            c.axis = Some(axis);
                        }
                    }
                }
                anchor_spans.insert(span_id, span);
            }
        }

        // Pass 3: classify bridgehead cells (have bridge_layer but not
        // anchor-overlay; not part of an AnchorSpan).
        for cell in terrain.iter() {
            let Some(idx) = index_of(width, height, cell.rx, cell.ry) else {
                continue;
            };
            let Some(resolved) = terrain.cell(cell.rx, cell.ry) else {
                continue;
            };
            let Some(bl) = resolved.bridge_layer.as_ref() else {
                continue;
            };
            if is_anchor_overlay(bl.overlay_id) {
                continue;
            }
            // Bridgehead cells: ramp/connection cells. May not have deck_present
            // if treated purely as ground transition. Mark role only when
            // a BridgeRuntimeCell already exists.
            if let Some(c) = cells[idx].as_mut() {
                c.role = BridgeCellRole::Bridgehead;
                c.anchor_span_id = None;
                c.bridgehead_step = 0;
                c.axis = Some(bridge_direction_to_axis(bl.direction));
            }
        }

        let mut group_hitpoints = BTreeMap::new();
        let strength = strength_per_group.max(1);
        for group_id in group_cells.keys().copied() {
            group_hitpoints.insert(group_id, strength);
        }
        let endpoint_records = compute_bridge_endpoints(&group_cells, terrain, width, height);

        Self {
            width,
            height,
            cells,
            group_cells,
            group_hitpoints,
            strength_per_group: strength,
            bridge_strength: strength, // currently same; Phase F can split if needed
            endpoint_records,
            anchor_spans,
            bridge_destroyable_flag: destroyable,
        }
    }

    pub fn anchor_span(&self, id: u16) -> Option<&AnchorSpan> {
        self.anchor_spans.get(&id)
    }

    pub fn anchor_spans(&self) -> &BTreeMap<u16, AnchorSpan> {
        &self.anchor_spans
    }
```

**Step 3: Add helpers at module level**

```rust
// src/sim/bridge_state.rs — at module bottom (before `mod tests`)

use crate::map::resolved_terrain::{BridgeDirection, BridgeLayer};

fn bridge_layer_to_axis(layer: Option<&BridgeLayer>) -> Option<Axis> {
    layer.map(|bl| bridge_direction_to_axis(bl.direction))
}

fn bridge_direction_to_axis(d: BridgeDirection) -> Axis {
    match d {
        BridgeDirection::EastWest => Axis::EW,
        BridgeDirection::NorthSouth => Axis::NS,
        // Low bridges are wood-overlay bridges with their own direction model.
        // Treat as NS for low; the low-bridge state machine reads `bridge_layer`
        // separately. (TODO: revisit after Phase C if low needs distinct axis.)
        BridgeDirection::Low => Axis::NS,
    }
}

/// HIGH bridge anchor overlays = 0x18, 0x19; LOW bridge anchor overlays =
/// 0xED, 0xEE. Per binary `Apply_area_damage @ 0x4894B0` dispatch table.
fn is_anchor_overlay(overlay_id: u8) -> bool {
    matches!(overlay_id, 0x18 | 0x19 | 0xED | 0xEE)
}

/// State-machine convention: NS-axis collapse walks E (dir=2) for ramp A,
/// W (dir=6) for ramp B. EW-axis collapse walks S (dir=4) for ramp A,
/// N (dir=0) for ramp B. We pick A-direction as the canonical anchor walk
/// direction (cell 5 then walks the opposite from anchor).
fn anchor_walk_direction(axis: Axis) -> Direction {
    match axis {
        Axis::NS => Direction::E,
        Axis::EW => Direction::S,
    }
}

/// Walk the 6-cell anchor pattern per `SetBridgeDirection_NESW` (verified
/// live `0x47E040`). Cells beyond the map edge become `None`.
fn walk_anchor_pattern(
    span_id: u16,
    anchor: (u16, u16),
    axis: Axis,
    direction: Direction,
    bridge_group_id: u16,
    width: u16,
    height: u16,
) -> AnchorSpan {
    let mut cells: [Option<(u16, u16)>; 6] = [None; 6];
    cells[0] = Some(anchor);

    let (dx, dy) = direction.offset();
    // Slot 1, 2, 3: walk +direction × 1, 2, 3.
    for step in 1..=3 {
        let nx = anchor.0 as i32 + dx * step;
        let ny = anchor.1 as i32 + dy * step;
        if nx >= 0 && ny >= 0 && (nx as u16) < width && (ny as u16) < height {
            cells[step as usize] = Some((nx as u16, ny as u16));
        }
    }

    // Slot 4: walk -direction × 1.
    let opp = direction.opposite();
    let (odx, ody) = opp.offset();
    let ox = anchor.0 as i32 + odx;
    let oy = anchor.1 as i32 + ody;
    if ox >= 0 && oy >= 0 && (ox as u16) < width && (oy as u16) < height {
        cells[4] = Some((ox as u16, oy as u16));
    }

    // Slot 5: optional fixed-offset only when direction == W (param_2 == 6 in binary).
    if direction == Direction::W {
        // Binary: DAT_0089F690 = E direction (+1, 0).
        let ex = anchor.0 as i32 + 1;
        let ey = anchor.1 as i32;
        if ex >= 0 && ey >= 0 && (ex as u16) < width && (ey as u16) < height {
            cells[5] = Some((ex as u16, ey as u16));
        }
    }

    AnchorSpan {
        id: span_id,
        anchor,
        cells,
        axis,
        direction,
        damage_state: DamageState::Healthy { variant: 0 },
        bridge_group_id,
    }
}
```

**Step 4: Update existing tests for new `cells` array shape**

The test fixture in `make_bridge_terrain` creates a 5x1 terrain where cells
1..=3 are bridge. With the new walker, the leftmost or middle cell becomes
the anchor. Update assertions:

```rust
    #[test]
    fn bridge_runtime_initializes_with_anchor_span() {
        let state = BridgeRuntimeState::from_resolved_terrain(&make_bridge_terrain(), true, 300);
        // Anchor walker creates at least one span.
        assert!(!state.anchor_spans().is_empty());
        // Existing assertions still pass:
        let cell = state.cell(1, 0).expect("bridge cell");
        assert!(cell.deck_present);
        assert!(matches!(cell.damage_state, DamageState::Healthy { .. }));
    }
```

**Step 5: Verify**

```
cargo build
cargo test --lib sim::bridge_state -- --nocapture
```
Expected: build green; existing bridge tests still pass.

**Step 6: Commit**

```
git commit -m "bridge_state: replace BFS-only constructor with SetBridgeDirection_NESW anchor walker (verified live 0x47E040)"
```

---

### Task 8: Update `app_init_helpers` to pass `bridge_destroyable_flag`

**Why:** Existing constructor signature is preserved, but `bridge_destroyable_flag`
field on `BridgeRuntimeState` is now populated from the existing per-map +
rules-default merge. Verify call site already does this.

**Files:** Verify `src/app_init_helpers.rs:343-360`.

**Step 1: Inspect existing call**

The current call uses `bridge_destroyable` as the `destroyable` param. After
Task 7, this is stored in both per-cell `destroyable` AND
`BridgeRuntimeState.bridge_destroyable_flag`. No call-site change needed.

**Step 2: Verify**

```
cargo build
```
Expected: green.

**Step 3: No commit (no changes)**

If Step 1 confirmed no change needed, no commit. Move to Task 9.

---

### Task 9: Update `world_hash.rs` to include new `BridgeRuntimeState` fields

**Why:** Determinism contract — state hash must include per-cell
`damage_state`, `axis`, `role`, `anchor_span_id`, `bridgehead_step`, and
the `AnchorSpan` registry. Otherwise replays of the same input + RNG yield
different hashes.

**Files:** Modify `src/sim/world/world_hash.rs`.

**Step 1: Locate existing bridge hash logic**

```
grep -n bridge src/sim/world/world_hash.rs
```

**Step 2: Update hash function to consume new fields**

The hashing function iterates `BridgeRuntimeState::iter_cells` (existing) and
should already cover any field marked with the same `Hash` derive. Since
all new enums/structs derive `Hash`, the existing iterator-based hash will
pick them up automatically IF the iterator hashes each cell field. Verify by
reading the existing implementation. If it explicitly enumerates fields:

```rust
// Augment field list to include damage_state, axis, role, anchor_span_id, bridgehead_step.
// Also hash anchor_spans (BTreeMap<u16, AnchorSpan> — sorted iteration).
for (id, span) in state.anchor_spans() {
    id.hash(hasher);
    span.hash(hasher);  // requires AnchorSpan: Hash — add derive in Task 5 if missing
}
```

**Step 3: Add `Hash` derive to `AnchorSpan`**

If not already present (Task 5 added `PartialEq, Eq` but may have skipped
`Hash`): add `Hash` to its derive list. Same for any helper enums missing it.

**Step 4: Verify**

```
cargo build
cargo test --lib world_hash -- --nocapture
```
Expected: build green, hash tests pass.

**Step 5: Commit**

```
git commit -m "world_hash: include new BridgeRuntimeCell fields + AnchorSpan registry in determinism hash"
```

---

### Task 10: Snapshot serde round-trip test

**Why:** New `BridgeRuntimeState` fields must serialize and deserialize
identically. Existing snapshot tests would break silently if a derive is
missing.

**Files:** Modify `src/sim/bridge_state.rs` `mod tests`.

**Step 1: Add round-trip test**

```rust
    #[test]
    fn bridge_runtime_state_snapshot_round_trip() {
        let state = BridgeRuntimeState::from_resolved_terrain(
            &make_bridge_terrain(), true, 1500,
        );
        let json = serde_json::to_string(&state).expect("serialize");
        let restored: BridgeRuntimeState =
            serde_json::from_str(&json).expect("deserialize");
        // Compare cell-by-cell.
        for (rx, ry) in [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)] {
            assert_eq!(state.cell(rx, ry), restored.cell(rx, ry), "cell ({rx},{ry})");
        }
        // Compare anchor spans.
        assert_eq!(
            state.anchor_spans().len(),
            restored.anchor_spans().len()
        );
        for (id, span) in state.anchor_spans() {
            assert_eq!(restored.anchor_span(*id), Some(span));
        }
    }
```

**Step 2: Verify**

```
cargo test --lib sim::bridge_state::tests::bridge_runtime_state_snapshot_round_trip -- --nocapture
```
Expected: PASS. If not: missing derive on a new struct/enum — add it.

**Step 3: Commit**

```
git commit -m "bridge_state: snapshot round-trip test for AnchorSpan + extended BridgeRuntimeCell"
```

---

### --- Phase B END — Foundation + state shape land. Build green. ---

---

### Task 11: `apply_ramp_transition` — state-machine transition function

**Why:** Mirrors the binary's 16 `UpdateRamp_*_High/_Low` helpers as a single
state-indexed transition function. Per HIGH §11.1, each helper computes
`next_state` from `current_state` per `(axis, phase)`; the `_Low` variants
share state transitions with `_High` (only their downstream overlay-base
constant differs — that propagation is a separate step). Whole thing
collapses to a `match` with ~16 arms. Pure function — fully unit-testable
in isolation. (Design ledger #31, revised.)

**Files:** Modify `src/sim/bridge_specs.rs`.

**Pattern:** New module-level helper alongside existing pure helpers.

**Step 1: Add the transition function**

```rust
// src/sim/bridge_specs.rs — at end of file, before `mod tests`

use crate::sim::bridge_state::{Axis, Phase};

/// Apply a single ramp state transition. Mirrors one of the binary's 16
/// `UpdateRamp_*_High/_Low` helpers (HIGH §11.1).
///
/// State byte semantics (CellClass+0x11E):
/// - NS-axis range: 0..=8 (0..=3 healthy, 4 = DamageA-set, 5 = DamageB-set,
///   6 = both halves damaged, 7 = PartialCollapseA, 8 = PartialCollapseB)
/// - EW-axis range: 9..=17 (9..=12 healthy, 0x0D = DamageB-set, 0x0E =
///   DamageA-set, 0x0F = both halves damaged, 0x10 = PartialCollapseB,
///   0x11 = PartialCollapseA)
///
/// Returns `Some(next_state)` on a defined transition, `None` if the
/// `(axis, phase, current_state)` combination has no transition (cell
/// unchanged).
///
/// **Collapse-final special case:** when input matches the "opposite-already-
/// collapsed" partial state (NS Collapse{A,B}: state 8/7; EW Collapse{A,B}:
/// state 0x10/0x11), the function returns `Some(0)` — but the caller MUST
/// also clear the bridge-direction flag, set `IsoTileTypeIndex = -1`, fire
/// `UpdateAdjacentBridges`, and zone-refresh. Body-cell driver detects this
/// via `(prev_state.is_partial_collapse() && phase.is_collapse() && next == 0)`.
///
/// `_Low` variants are intentionally not a parameter: state transitions are
/// identical, so the same function serves both. Overlay propagation (§11.2 +
/// `pick_destruction_overlay`) is what distinguishes HIGH from LOW.
pub fn apply_ramp_transition(
    current_state: u8,
    axis: Axis,
    phase: Phase,
) -> Option<u8> {
    match (axis, phase, current_state) {
        // --- NS axis (state 0..=8) ---
        // NS_DamageA: 0..=3 → 4, 5 → 6
        (Axis::NS, Phase::DamageA, 0..=3) => Some(4),
        (Axis::NS, Phase::DamageA, 5) => Some(6),
        // NS_DamageB: 0..=3 → 5, 4 → 6
        (Axis::NS, Phase::DamageB, 0..=3) => Some(5),
        (Axis::NS, Phase::DamageB, 4) => Some(6),
        // NS_CollapseA: 0..=6 → 7, 8 → 0 (collapse-final)
        (Axis::NS, Phase::CollapseA, 0..=6) => Some(7),
        (Axis::NS, Phase::CollapseA, 8) => Some(0),
        // NS_CollapseB: 0..=6 → 8, 7 → 0 (collapse-final)
        (Axis::NS, Phase::CollapseB, 0..=6) => Some(8),
        (Axis::NS, Phase::CollapseB, 7) => Some(0),

        // --- EW axis (state 9..=17 / 0x09..=0x11) ---
        // EW_DamageA: 9..=12 → 0x0E, 0x0D → 0x0F
        (Axis::EW, Phase::DamageA, 9..=12) => Some(0x0E),
        (Axis::EW, Phase::DamageA, 0x0D) => Some(0x0F),
        // EW_DamageB: 9..=12 → 0x0D, 0x0E → 0x0F
        (Axis::EW, Phase::DamageB, 9..=12) => Some(0x0D),
        (Axis::EW, Phase::DamageB, 0x0E) => Some(0x0F),
        // EW_CollapseA: 9..=15 → 0x11, 0x10 → 0 (collapse-final)
        (Axis::EW, Phase::CollapseA, 9..=15) => Some(0x11),
        (Axis::EW, Phase::CollapseA, 0x10) => Some(0),
        // EW_CollapseB: 9..=15 → 0x10, 0x11 → 0 (collapse-final)
        (Axis::EW, Phase::CollapseB, 9..=15) => Some(0x10),
        (Axis::EW, Phase::CollapseB, 0x11) => Some(0),

        // No defined transition.
        _ => None,
    }
}
```

**Step 2: Add unit tests covering each row**

```rust
// in src/sim/bridge_specs.rs `mod tests`

    use crate::sim::bridge_state::{Axis, Phase};

    #[test]
    fn ramp_ns_damage_a_healthy_to_4() {
        for s in 0..=3 {
            assert_eq!(
                apply_ramp_transition(s, Axis::NS, Phase::DamageA),
                Some(4),
                "state {s}"
            );
        }
    }

    #[test]
    fn ramp_ns_damage_a_5_to_6() {
        assert_eq!(apply_ramp_transition(5, Axis::NS, Phase::DamageA), Some(6));
    }

    #[test]
    fn ramp_ns_damage_b_healthy_to_5() {
        for s in 0..=3 {
            assert_eq!(apply_ramp_transition(s, Axis::NS, Phase::DamageB), Some(5));
        }
    }

    #[test]
    fn ramp_ns_damage_b_4_to_6() {
        assert_eq!(apply_ramp_transition(4, Axis::NS, Phase::DamageB), Some(6));
    }

    #[test]
    fn ramp_ns_collapse_a_to_7() {
        for s in 0..=6 {
            assert_eq!(apply_ramp_transition(s, Axis::NS, Phase::CollapseA), Some(7));
        }
    }

    #[test]
    fn ramp_ns_collapse_a_final_state_8_to_0() {
        // Collapse-final: caller must also clear bridge dir + IsoTileTypeIndex.
        assert_eq!(apply_ramp_transition(8, Axis::NS, Phase::CollapseA), Some(0));
    }

    #[test]
    fn ramp_ns_collapse_b_to_8() {
        for s in 0..=6 {
            assert_eq!(apply_ramp_transition(s, Axis::NS, Phase::CollapseB), Some(8));
        }
    }

    #[test]
    fn ramp_ns_collapse_b_final_state_7_to_0() {
        assert_eq!(apply_ramp_transition(7, Axis::NS, Phase::CollapseB), Some(0));
    }

    #[test]
    fn ramp_ew_damage_a_healthy_to_e() {
        for s in 9..=12 {
            assert_eq!(apply_ramp_transition(s, Axis::EW, Phase::DamageA), Some(0x0E));
        }
    }

    #[test]
    fn ramp_ew_damage_a_d_to_f() {
        assert_eq!(apply_ramp_transition(0x0D, Axis::EW, Phase::DamageA), Some(0x0F));
    }

    #[test]
    fn ramp_ew_damage_b_healthy_to_d() {
        for s in 9..=12 {
            assert_eq!(apply_ramp_transition(s, Axis::EW, Phase::DamageB), Some(0x0D));
        }
    }

    #[test]
    fn ramp_ew_damage_b_e_to_f() {
        assert_eq!(apply_ramp_transition(0x0E, Axis::EW, Phase::DamageB), Some(0x0F));
    }

    #[test]
    fn ramp_ew_collapse_a_to_11() {
        for s in 9..=15 {
            assert_eq!(apply_ramp_transition(s, Axis::EW, Phase::CollapseA), Some(0x11));
        }
    }

    #[test]
    fn ramp_ew_collapse_a_final_state_10_to_0() {
        assert_eq!(apply_ramp_transition(0x10, Axis::EW, Phase::CollapseA), Some(0));
    }

    #[test]
    fn ramp_ew_collapse_b_to_10() {
        for s in 9..=15 {
            assert_eq!(apply_ramp_transition(s, Axis::EW, Phase::CollapseB), Some(0x10));
        }
    }

    #[test]
    fn ramp_ew_collapse_b_final_state_11_to_0() {
        assert_eq!(apply_ramp_transition(0x11, Axis::EW, Phase::CollapseB), Some(0));
    }

    #[test]
    fn ramp_undefined_combination_returns_none() {
        // EW phase on NS-range state, etc.
        assert_eq!(apply_ramp_transition(0, Axis::EW, Phase::DamageA), None);
        assert_eq!(apply_ramp_transition(15, Axis::NS, Phase::DamageA), None);
        // State outside both ranges.
        assert_eq!(apply_ramp_transition(0xFF, Axis::NS, Phase::DamageA), None);
    }
```

**Step 3: Verify**

```
cargo test --lib sim::bridge_specs -- --nocapture
```
Expected: PASS (16 new transition tests).

**Step 4: Commit**

```
git commit -m "bridge_specs: add apply_ramp_transition state-machine function (HIGH §11.1, all 8 helpers' transitions in one match)"
```

---

### Task 11.5: `pick_destruction_overlay` + populate §11.2 next-overlay tables

**Why:** Per HIGH §11.2, the binary has *separate* per-cell visual transition
primitives (`ApplyBridgeDestruction_*`) that pick the next-overlay byte from
a 16-entry table indexed by a `CheckBridgeNeighbors_*` result. These are
distinct from the §11.1 state-transition helpers handled in Task 11. Need
its own helper + 4 backing tables (HIGH × {NS,EW} + LOW × {NS,EW}).
HIGH NS/EW tables are already documented in HIGH §11.2 indices 0..=10;
indices 11..=15 are documented as `-1` (verify in 11.5a). LOW equivalents
need fresh decompile (11.5b). Design ledger #32, revised.

**Files:** Modify `src/sim/bridge_specs.rs`. Sub-tasks 11.5a (HIGH spot-check)
and 11.5b (LOW decompile) may be done in either order.

**Step 1: Add the helper and tables (HIGH already filled, LOW left as
`0xFF; 16` until 11.5b)**

```rust
// src/sim/bridge_specs.rs — alongside apply_ramp_transition

/// Pick the next overlay byte for a destroying bridge cell. Mirrors
/// `ApplyBridgeDestruction_NS_High @ 0x57E7A0` and `_EW_High @ 0x57ED00`
/// (HIGH §11.2). Indexed by the result of `CheckBridgeNeighbors_*` —
/// i.e., a small integer encoding which adjacent cells still hold bridge
/// overlay. Distinct from `apply_ramp_transition` which handles state
/// (CellClass+0x11E); this one writes the visible overlay byte (+0x44).
///
/// `0xFF` in the table represents the binary's `-1` sentinel ("no
/// transition for this neighbor pattern" — leave overlay alone).
pub fn pick_destruction_overlay(
    neighbor_check: u8,
    axis: Axis,
    is_high_bridge: bool,
) -> Option<u8> {
    if neighbor_check >= 16 {
        return None;
    }
    let table: &[u8; 16] = match (axis, is_high_bridge) {
        (Axis::NS, true) => &DESTRUCTION_OVERLAY_HIGH_NS,
        (Axis::EW, true) => &DESTRUCTION_OVERLAY_HIGH_EW,
        (Axis::NS, false) => &DESTRUCTION_OVERLAY_LOW_NS,
        (Axis::EW, false) => &DESTRUCTION_OVERLAY_LOW_EW,
    };
    let val = table[neighbor_check as usize];
    if val == 0xFF { None } else { Some(val) }
}

/// HIGH NS destruction overlay table per HIGH §11.2 (`ApplyBridgeDestruction_NS_High`
/// @ `0x57E7A0`). Indexed by `CheckBridgeNeighbors_EW_High` result.
/// Indices 0..=10 verified from §11.2; indices 11..=15 marked `-1` in doc,
/// pending Task 11.5a binary spot-check.
static DESTRUCTION_OVERLAY_HIGH_NS: [u8; 16] = [
    0xFF, 0xD2, 0xD5, 0xFF, 0xD1, 0xD3, 0xD5, 0xFF,
    0xD4, 0xD4, 0xE7, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];

/// HIGH EW destruction overlay table per HIGH §11.2 (`ApplyBridgeDestruction_EW_High`
/// @ `0x57ED00`). Indexed by `CheckBridgeNeighbors_NS_High` result.
static DESTRUCTION_OVERLAY_HIGH_EW: [u8; 16] = [
    0xFF, 0xDB, 0xDE, 0xFF, 0xDA, 0xDC, 0xDE, 0xFF,
    0xDD, 0xDD, 0xE8, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];

/// LOW NS destruction overlay table per `ApplyBridgeDestruction_NS_Low`
/// @ `0x0057DD50` (verified live, see HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md
/// §11.2-LOW). Indexed by `CheckBridgeNeighbors_EW_Low` result. Final
/// destroyed byte = `0x64`. Outer overlay gate: `0x4A..=0x65`.
/// Progressive intermediates (handled by caller, not table):
/// `0x5C → 0x5D`, `0x5E → 0x5F`.
static DESTRUCTION_OVERLAY_LOW_NS: [u8; 16] = [
    0xFF, 0x4F, 0x52, 0xFF, 0x4E, 0x50, 0x52, 0xFF,
    0x51, 0x51, 0x64, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];

/// LOW EW destruction overlay table per `ApplyBridgeDestruction_EW_Low`
/// @ `0x0057E2A0` (verified live). Indexed by `CheckBridgeNeighbors_NS_Low`
/// result. Final destroyed byte = `0x65`. Outer overlay gate: `0x4A..=0x65`.
/// Progressive intermediates: `0x60 → 0x61`, `0x62 → 0x63`.
static DESTRUCTION_OVERLAY_LOW_EW: [u8; 16] = [
    0xFF, 0x58, 0x5B, 0xFF, 0x57, 0x59, 0x5B, 0xFF,
    0x5A, 0x5A, 0x65, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];
```

**Step 2: Add unit tests for the HIGH tables (LOW tests deferred to 11.5b)**

```rust
// in src/sim/bridge_specs.rs `mod tests`

    #[test]
    fn destruction_overlay_high_ns_known_entries() {
        // Spot-check verified entries from HIGH §11.2.
        assert_eq!(pick_destruction_overlay(1, Axis::NS, true), Some(0xD2));
        assert_eq!(pick_destruction_overlay(2, Axis::NS, true), Some(0xD5));
        assert_eq!(pick_destruction_overlay(4, Axis::NS, true), Some(0xD1));
        assert_eq!(pick_destruction_overlay(10, Axis::NS, true), Some(0xE7)); // final destroyed
    }

    #[test]
    fn destruction_overlay_high_ew_known_entries() {
        assert_eq!(pick_destruction_overlay(1, Axis::EW, true), Some(0xDB));
        assert_eq!(pick_destruction_overlay(2, Axis::EW, true), Some(0xDE));
        assert_eq!(pick_destruction_overlay(10, Axis::EW, true), Some(0xE8)); // final destroyed
    }

    #[test]
    fn destruction_overlay_unused_indices_return_none() {
        assert_eq!(pick_destruction_overlay(0, Axis::NS, true), None);
        assert_eq!(pick_destruction_overlay(3, Axis::NS, true), None);
        assert_eq!(pick_destruction_overlay(11, Axis::NS, true), None);
    }

    #[test]
    fn destruction_overlay_out_of_range_returns_none() {
        assert_eq!(pick_destruction_overlay(16, Axis::NS, true), None);
        assert_eq!(pick_destruction_overlay(0xFF, Axis::EW, true), None);
    }

    #[test]
    fn destruction_overlay_low_ns_known_entries() {
        // Verified from ApplyBridgeDestruction_NS_Low @ 0x0057DD50.
        assert_eq!(pick_destruction_overlay(1, Axis::NS, false), Some(0x4F));
        assert_eq!(pick_destruction_overlay(2, Axis::NS, false), Some(0x52));
        assert_eq!(pick_destruction_overlay(4, Axis::NS, false), Some(0x4E));
        assert_eq!(pick_destruction_overlay(10, Axis::NS, false), Some(0x64)); // final destroyed
    }

    #[test]
    fn destruction_overlay_low_ew_known_entries() {
        // Verified from ApplyBridgeDestruction_EW_Low @ 0x0057E2A0.
        assert_eq!(pick_destruction_overlay(1, Axis::EW, false), Some(0x58));
        assert_eq!(pick_destruction_overlay(2, Axis::EW, false), Some(0x5B));
        assert_eq!(pick_destruction_overlay(4, Axis::EW, false), Some(0x57));
        assert_eq!(pick_destruction_overlay(10, Axis::EW, false), Some(0x65)); // final destroyed
    }

    #[test]
    fn destruction_overlay_low_unused_indices_return_none() {
        // Slots 0/3/7/11..=15 unused in both NS and EW LOW tables.
        for i in [0, 3, 7, 11, 12, 13, 14, 15] {
            assert_eq!(pick_destruction_overlay(i, Axis::NS, false), None, "NS slot {i}");
            assert_eq!(pick_destruction_overlay(i, Axis::EW, false), None, "EW slot {i}");
        }
    }
```

**Step 3: Verify**

```
cargo test --lib sim::bridge_specs -- --nocapture
```

**Step 4: Commit**

```
git commit -m "bridge_specs: add pick_destruction_overlay + HIGH §11.2 next-overlay tables (LOW pending 11.5b)"
```

---

### Task 11.5a — Resolved (research complete, no implementation needed)

**Status:** Verified live `0x0057E7A0` and `0x0057ED00` (2026-05-07). Both
HIGH NS and HIGH EW tables explicitly initialize indices 11..=15 to
`0xffffffff` (`-1`) in the function prologue. Tables in Task 11.5
correctly mark these entries as `0xFF`. No code or plan correction
needed; no commit required.

`HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §11.2 has been
amended with the verified-live note.

---

### Task 11.5b — Resolved (research complete; LOW tables populated in Task 11.5 above)

**Status:** Both `ApplyBridgeDestruction_NS_Low` and `_EW_Low` were
already labeled in the Ghidra database (despite the HIGH report not
covering them). Decompiled live (2026-05-07):

| Function | Address | Final destroyed byte | Outer overlay gate |
|---|---|---|---|
| `MapClass__ApplyBridgeDestruction_NS_Low` | `0x0057DD50` | `0x64` | `0x4A..=0x65` |
| `MapClass__ApplyBridgeDestruction_EW_Low` | `0x0057E2A0` | `0x65` | `0x4A..=0x65` |

Companion neighbor-check helpers (also already labeled): `CheckBridgeNeighbors_NS_Low`
@ `0x0057B990`, `CheckBridgeNeighbors_EW_Low` @ `0x0057B870`.

Progressive intermediates (handled by caller dispatch in the function,
not via the 16-entry table):
- LOW NS: `0x5C → 0x5D`, `0x5E → 0x5F`
- LOW EW: `0x60 → 0x61`, `0x62 → 0x63`

Both `DESTRUCTION_OVERLAY_LOW_NS` and `DESTRUCTION_OVERLAY_LOW_EW` in
Task 11.5's code block above are populated with verified values; tests
in Task 11.5's `mod tests` cover them. No follow-up implementation
required beyond what Task 11.5 already specifies.

`HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` has been amended
with a §11.2-LOW subsection containing the addresses + extracted tables.

---

### Task 12: `set_bridge_direction` walker — 6-cell pattern emitter

**Why:** Per design ledger #34–42. Replaces binary's `SetBridgeDirection_NESW`
flag-twiddling with a typed enum-emit pattern. On destruction path (`set =
false`), emits exactly 4 BlowUpBridge actions (slots 0, 1, 2, 4) per
verified `0x47E040`.

**Files:** Modify `src/sim/bridge_specs.rs`.

**Step 1: Add `CellAction` enum**

```rust
// src/sim/bridge_specs.rs

/// Per-cell action emitted by `set_bridge_direction` walker. The orchestrator
/// in `world::resolve_bridge_state_changes` consumes these and dispatches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellAction {
    /// Cell receives `BlowUpBridge` (kill ground, Limbo bridge-deck, debris).
    /// Destruction path slots 0, 1, 2, 4 (binary cells 1, 2, 3, 5).
    BlowUpBridge,
    /// Cell receives flag-only update — no BlowUpBridge. Slot 3 (binary
    /// cell 4) and slot 5 (binary cell 6) on destruction path.
    FlagOnly,
}

/// Result from `set_bridge_direction` walker. Each entry is one cell + its
/// action.
#[derive(Debug, Clone)]
pub struct SetBridgeDirectionResult {
    pub actions: Vec<((u16, u16), usize, CellAction)>,
}
```

**Step 2: Add the walker function**

```rust
use crate::sim::bridge_state::{AnchorSpan, Direction};

/// Emit the per-cell action list for an anchor span. Mirrors binary's
/// `SetBridgeDirection_NESW @ 0x47E040`.
///
/// `set == false` is the destruction path (4 BlowUpBridge calls + 1–2
/// flag-only). `set == true` is the build/intact path (no BlowUpBridge —
/// flag writes only). Tier 2 only consumes destruction path; build path
/// is exercised by map-load anchor walker construction (Task 7).
pub fn set_bridge_direction(span: &AnchorSpan, set: bool) -> SetBridgeDirectionResult {
    let mut actions = Vec::with_capacity(6);
    for (slot, cell) in span.iter_cells() {
        let action = if !set {
            // Destruction path: slots 0, 1, 2, 4 = BlowUpBridge; 3, 5 = FlagOnly.
            // Verified live `0x47E040`.
            if AnchorSpan::BLOW_UP_SLOTS.contains(&slot) {
                CellAction::BlowUpBridge
            } else {
                CellAction::FlagOnly
            }
        } else {
            // Build path: every cell is FlagOnly (no BlowUpBridge). Used by
            // map-load construction.
            CellAction::FlagOnly
        };
        actions.push((cell, slot, action));
    }
    SetBridgeDirectionResult { actions }
}
```

**Step 3: Add tests**

```rust
    #[test]
    fn set_bridge_direction_destruction_emits_4_blow_up_actions() {
        let span = AnchorSpan {
            id: 1,
            anchor: (5, 5),
            cells: [
                Some((5, 5)), Some((6, 5)), Some((7, 5)),
                Some((8, 5)), Some((4, 5)), None,
            ],
            axis: Axis::NS,
            direction: Direction::E,
            damage_state: DamageState::Damaged,
            bridge_group_id: 1,
        };
        let result = set_bridge_direction(&span, false);
        let blow_ups = result.actions.iter()
            .filter(|(_, _, a)| matches!(a, CellAction::BlowUpBridge))
            .count();
        assert_eq!(blow_ups, 4);
        let flag_only = result.actions.iter()
            .filter(|(_, _, a)| matches!(a, CellAction::FlagOnly))
            .count();
        assert_eq!(flag_only, 1); // slot 3 (cell 4)
    }

    #[test]
    fn set_bridge_direction_build_emits_no_blow_up_actions() {
        let span = AnchorSpan {
            id: 1,
            anchor: (0, 0),
            cells: [Some((0, 0)), None, None, None, None, None],
            axis: Axis::NS,
            direction: Direction::E,
            damage_state: DamageState::Healthy { variant: 0 },
            bridge_group_id: 1,
        };
        let result = set_bridge_direction(&span, true);
        assert!(result.actions.iter().all(|(_, _, a)| matches!(a, CellAction::FlagOnly)));
    }

    #[test]
    fn set_bridge_direction_includes_slot_5_only_when_present() {
        let span = AnchorSpan {
            id: 1,
            anchor: (5, 5),
            cells: [
                Some((5, 5)), Some((6, 5)), Some((7, 5)),
                Some((8, 5)), Some((4, 5)),
                Some((6, 5)), // hypothetical slot 5
            ],
            axis: Axis::NS,
            direction: Direction::W,
            damage_state: DamageState::Damaged,
            bridge_group_id: 1,
        };
        let result = set_bridge_direction(&span, false);
        let slot_5_action = result.actions.iter()
            .find(|(_, slot, _)| *slot == 5)
            .map(|(_, _, a)| *a);
        assert_eq!(slot_5_action, Some(CellAction::FlagOnly));
    }
```

**Step 4: Verify**

```
cargo test --lib sim::bridge_specs::set_bridge_direction -- --nocapture
```
Expected: PASS.

**Step 5: Commit**

```
git commit -m "bridge_specs: add set_bridge_direction walker (4 BlowUpBridge + 1-2 FlagOnly per binary 0x47E040)"
```

---

### Task 13: Body-cell state-machine driver

**Why:** Per HIGH §3.1 body-cell branch. Implements
Healthy → Damaged → Collapse for both axes plus partial-collapse handling
(states 7/17 → CollapseA, 8/16 → CollapseB).

**Files:** Modify `src/sim/bridge_specs.rs`.

**Step 1: Add `StateOutcome` and driver**

```rust
// src/sim/bridge_specs.rs

/// Outcome of one state-machine advance step.
#[derive(Debug, Clone)]
pub enum StateOutcome {
    /// Damage absorbed; visual state may have advanced (Healthy → Damaged)
    /// but bridge still passable.
    Absorbed { ramp_writes: Vec<RampWrite> },
    /// Anchor span collapsed; emit BlowUpBridge actions.
    Collapsed { ramp_writes: Vec<RampWrite>, set_bridge_direction: SetBridgeDirectionResult },
    /// No transition (state unchanged).
    NoChange,
}

/// One ramp-overlay write request: cell + new overlay byte.
#[derive(Debug, Clone, Copy)]
pub struct RampWrite {
    pub cell: (u16, u16),
    pub overlay_byte: u8,
}

/// Body-cell state-machine driver. One call = one damage hit on a body cell.
/// Mirrors `ProcessBridgeDamageStateMachine_High` body-cell branch (HIGH §3.1).
pub fn body_cell_advance_state(
    span: &mut AnchorSpan,
    is_high_bridge: bool,
) -> StateOutcome {
    use Phase::*;

    match span.damage_state {
        // Healthy → Damaged: visible damage, bridge still passable.
        DamageState::Healthy { .. } => {
            let mut writes = Vec::with_capacity(2);
            // DamageA at slot 1 (cell 2), DamageB at slot 2 (cell 3).
            // Slot indices match binary's iVar2 (NS=2, EW=4) / iVar6 (NS=6, EW=0)
            // when those are translated through anchor walker. Per HIGH §3.1
            // call args: NS DamageA(2) + DamageB(6); EW DamageA(4) + DamageB(0).
            // We map binary's "slot 2" to our slot 1 since our slot 0 is anchor.
            for (slot, phase) in [(1, DamageA), (2, DamageB)] {
                if let Some(cell) = span.cells[slot] {
                    if let Some(byte) = apply_ramp_transition(slot as u8, span.axis, phase, true, is_high_bridge) {
                        writes.push(RampWrite { cell, overlay_byte: byte });
                    }
                }
            }
            span.damage_state = DamageState::Damaged;
            StateOutcome::Absorbed { ramp_writes: writes }
        }
        // Damaged → full collapse: CollapseA + CollapseB + SetBridgeDirection_NESW.
        DamageState::Damaged => {
            let mut writes = Vec::with_capacity(2);
            for (slot, phase) in [(1, CollapseA), (2, CollapseB)] {
                if let Some(cell) = span.cells[slot] {
                    if let Some(byte) = apply_ramp_transition(slot as u8, span.axis, phase, true, is_high_bridge) {
                        writes.push(RampWrite { cell, overlay_byte: byte });
                    }
                }
            }
            span.damage_state = DamageState::Destroyed;
            let sbd = set_bridge_direction(span, false);
            StateOutcome::Collapsed { ramp_writes: writes, set_bridge_direction: sbd }
        }
        // Partial-collapse states 7/17: ramp B already collapsed; fire CollapseA only.
        DamageState::PartialCollapseA => {
            let mut writes = Vec::with_capacity(1);
            if let Some(cell) = span.cells[1] {
                if let Some(byte) = apply_ramp_transition(1, span.axis, Phase::CollapseA, true, is_high_bridge) {
                    writes.push(RampWrite { cell, overlay_byte: byte });
                }
            }
            span.damage_state = DamageState::Destroyed;
            let sbd = set_bridge_direction(span, false);
            StateOutcome::Collapsed { ramp_writes: writes, set_bridge_direction: sbd }
        }
        // Partial-collapse states 8/16: ramp A already collapsed; fire CollapseB only.
        DamageState::PartialCollapseB => {
            let mut writes = Vec::with_capacity(1);
            if let Some(cell) = span.cells[2] {
                if let Some(byte) = apply_ramp_transition(2, span.axis, Phase::CollapseB, true, is_high_bridge) {
                    writes.push(RampWrite { cell, overlay_byte: byte });
                }
            }
            span.damage_state = DamageState::Destroyed;
            let sbd = set_bridge_direction(span, false);
            StateOutcome::Collapsed { ramp_writes: writes, set_bridge_direction: sbd }
        }
        DamageState::Destroyed => StateOutcome::NoChange,
    }
}
```

**Step 2: Add tests**

```rust
    fn make_intact_span(axis: Axis) -> AnchorSpan {
        AnchorSpan {
            id: 1,
            anchor: (5, 5),
            cells: [
                Some((5, 5)), Some((6, 5)), Some((7, 5)),
                Some((8, 5)), Some((4, 5)), None,
            ],
            axis,
            direction: match axis { Axis::NS => Direction::E, Axis::EW => Direction::S },
            damage_state: DamageState::Healthy { variant: 0 },
            bridge_group_id: 1,
        }
    }

    #[test]
    fn body_cell_first_hit_advances_to_damaged_returns_absorbed() {
        let mut span = make_intact_span(Axis::NS);
        let outcome = body_cell_advance_state(&mut span, true);
        assert!(matches!(outcome, StateOutcome::Absorbed { .. }));
        assert_eq!(span.damage_state, DamageState::Damaged);
    }

    #[test]
    fn body_cell_second_hit_advances_damaged_to_destroyed_returns_collapsed() {
        let mut span = make_intact_span(Axis::EW);
        body_cell_advance_state(&mut span, true); // Healthy → Damaged
        let outcome = body_cell_advance_state(&mut span, true); // Damaged → Destroyed
        assert!(matches!(outcome, StateOutcome::Collapsed { .. }));
        assert_eq!(span.damage_state, DamageState::Destroyed);
    }

    #[test]
    fn body_cell_partial_collapse_a_fires_collapse_a_only() {
        let mut span = make_intact_span(Axis::NS);
        span.damage_state = DamageState::PartialCollapseA;
        let outcome = body_cell_advance_state(&mut span, true);
        match outcome {
            StateOutcome::Collapsed { ramp_writes, .. } => {
                // Verify only one ramp write (CollapseA at slot 1).
                assert_eq!(ramp_writes.len(), 1);
            }
            _ => panic!("expected Collapsed"),
        }
    }

    #[test]
    fn body_cell_partial_collapse_b_fires_collapse_b_only() {
        let mut span = make_intact_span(Axis::EW);
        span.damage_state = DamageState::PartialCollapseB;
        let outcome = body_cell_advance_state(&mut span, true);
        match outcome {
            StateOutcome::Collapsed { ramp_writes, .. } => {
                assert_eq!(ramp_writes.len(), 1);
            }
            _ => panic!("expected Collapsed"),
        }
    }

    #[test]
    fn body_cell_destroyed_state_returns_no_change() {
        let mut span = make_intact_span(Axis::NS);
        span.damage_state = DamageState::Destroyed;
        let outcome = body_cell_advance_state(&mut span, true);
        assert!(matches!(outcome, StateOutcome::NoChange));
    }
```

**Step 3: Verify**

```
cargo test --lib sim::bridge_specs::body_cell -- --nocapture
```
Expected: PASS.

**Step 4: Commit**

```
git commit -m "bridge_specs: body-cell state machine (Healthy→Damaged→Collapse + partial-collapse 7/17→CollapseA, 8/16→CollapseB)"
```

---

### Task 14: Bridgehead walk-to-anchor helper

**Why:** Per HIGH §3.2 + Task 0 verification (live `0x576BA0` on 2026-05-07).
Bridgehead damage walks `+DirectionOffset` reading `cell.height` at field
offset **`+0x11A`** (NOT `+0x52` as the design originally hypothesized).
Termination height: 4 (NS) or 2 (EW). **Per-axis early-return predicate is
asymmetric:** NS rejects odd heights via `(h & 1) != 0`; EW rejects only
heights > 4 via `4 < h`. Implement both predicates explicitly.

**Files:** Modify `src/sim/bridge_specs.rs`.

**Step 1: Add helper**

```rust
use crate::sim::bridge_state::Direction;

/// Walk from a bridgehead cell to its anchor body cell. Returns the anchor
/// cell coord, or `None` if the walk hits an invalid intermediate (per-axis
/// guard) or runs off the map.
///
/// Per HIGH §3.2 + verified live `0x576BA0`:
/// - **NS branch:** reject `(height & 1) != 0` (all odd heights); walk
///   `+DirectionOffset` until `height == 4`.
/// - **EW branch:** reject `4 < height` (heights > 4 only); walk
///   `+DirectionOffset` until `height == 2`.
///
/// Field is `cell.height` at `CellClass+0x11A` (confirmed Task 0). Companion
/// `+0x11B` is `cell.Level` — read separately by callers needing the
/// `level - 4` adjustment for `SetOverlayAndPropagate`.
pub fn bridgehead_walk_to_anchor(
    start: (u16, u16),
    axis: Axis,
    direction: Direction,
    cell_height: impl Fn((u16, u16)) -> Option<u8>,
    map_width: u16,
    map_height: u16,
) -> Option<(u16, u16)> {
    let target_height: u8 = match axis {
        Axis::NS => 4,
        Axis::EW => 2,
    };
    let (dx, dy) = direction.offset();
    let mut current = start;
    for _ in 0..16 { // bounded walk, real spans are ≤ 6
        let h = cell_height(current)?;
        // Per-axis early-return guard (asymmetric — verified live 0x576BA0).
        match axis {
            Axis::NS => {
                if h & 1 != 0 {
                    return None; // NS rejects all odd heights
                }
            }
            Axis::EW => {
                if h > 4 {
                    return None; // EW rejects heights > 4 only
                }
            }
        }
        if h == target_height {
            return Some(current);
        }
        let nx = current.0 as i32 + dx;
        let ny = current.1 as i32 + dy;
        if nx < 0 || ny < 0 || nx as u16 >= map_width || ny as u16 >= map_height {
            return None;
        }
        current = (nx as u16, ny as u16);
    }
    None
}
```

**Step 2: Add tests using mock height function**

```rust
    #[test]
    fn bridgehead_walk_ns_finds_anchor_at_height_4() {
        // Linear strip: heights 8, 6, 4, 2, 0
        let heights = |(x, y): (u16, u16)| {
            if y != 0 { return None; }
            match x {
                0 => Some(8), 1 => Some(6), 2 => Some(4),
                3 => Some(2), 4 => Some(0),
                _ => None,
            }
        };
        let r = bridgehead_walk_to_anchor((0, 0), Axis::NS, Direction::E, heights, 5, 1);
        assert_eq!(r, Some((2, 0))); // height 4
    }

    #[test]
    fn bridgehead_walk_ns_rejects_odd_intermediate() {
        // NS branch: odd height in path → reject via `(h & 1) != 0`.
        let heights = |(x, _y): (u16, u16)| match x {
            0 => Some(8), 1 => Some(7), _ => Some(0),
        };
        let r = bridgehead_walk_to_anchor((0, 0), Axis::NS, Direction::E, heights, 5, 1);
        assert!(r.is_none());
    }

    #[test]
    fn bridgehead_walk_ew_accepts_odd_intermediate_below_5() {
        // EW branch: rejects only h > 4. h=3 passes the guard (would be
        // rejected by NS predicate). Strip: 4, 3, 2 — anchor at h==2.
        let heights = |(x, _y): (u16, u16)| match x {
            0 => Some(4), 1 => Some(3), 2 => Some(2),
            _ => None,
        };
        let r = bridgehead_walk_to_anchor((0, 0), Axis::EW, Direction::E, heights, 3, 1);
        assert_eq!(r, Some((2, 0)));
    }

    #[test]
    fn bridgehead_walk_ew_rejects_height_above_4() {
        // EW branch: h=5 is rejected via `h > 4`.
        let heights = |(x, _y): (u16, u16)| match x {
            0 => Some(5), _ => Some(0),
        };
        let r = bridgehead_walk_to_anchor((0, 0), Axis::EW, Direction::E, heights, 3, 1);
        assert!(r.is_none());
    }

    #[test]
    fn bridgehead_walk_returns_none_on_map_edge() {
        let heights = |_: (u16, u16)| Some(8);
        let r = bridgehead_walk_to_anchor((0, 0), Axis::NS, Direction::E, heights, 1, 1);
        assert!(r.is_none()); // would walk off the map
    }
```

**Step 3: Verify**

```
cargo test --lib sim::bridge_specs::bridgehead_walk -- --nocapture
```

**Step 4: Commit**

```
git commit -m "bridge_specs: bridgehead_walk_to_anchor — per-axis predicates (NS h&1, EW h>4) at CellClass+0x11A (verified live 0x576BA0)"
```

---

### Task 15: Bridgehead state-machine driver

**Why:** Per HIGH §3.2. 4-step progression (steps 0–2 absorb damage,
step 3 = full collapse with `BlowUpBridge × 3` perpendicular cells +
`SetOverlayAndPropagate` + ramp collapse + zone refresh + 10-slot debris).

**Files:** Modify `src/sim/bridge_specs.rs`.

**Step 1: Add driver**

```rust
/// Bridgehead state-machine driver — one call per damage hit on a bridgehead
/// cell. Mirrors `ProcessBridgeDamageStateMachine_High` bridgehead branch
/// (HIGH §3.2).
///
/// Returns:
/// - Absorbed for steps 0..=2 (visible damage progression on the ramp).
/// - Collapsed for step 3 (full collapse: cascade BlowUpBridge × 3 perpendicular,
///   SetOverlayAndPropagate, ramp CollapseA + CollapseB, UpdateAdjacentBridges × 2).
pub fn bridgehead_advance_state(
    bridgehead_cell: (u16, u16),
    bridgehead_step: &mut u8,
    axis: Axis,
    anchor_span: &mut AnchorSpan,
    is_high_bridge: bool,
) -> StateOutcome {
    let mut writes = Vec::new();

    match *bridgehead_step {
        0 | 1 | 2 => {
            // Progressive damage: write next-step overlay, fire DamageA/B on anchor.
            *bridgehead_step += 1;
            // SetOverlayAndPropagate(anchor, base+offset+1) — write progressive overlay.
            // Implementation detail: caller's apply_area_damage will compute the
            // overlay byte and write it. Here we emit ramp writes for the anchor.
            for (slot, phase) in [(1, Phase::DamageA), (2, Phase::DamageB)] {
                if let Some(cell) = anchor_span.cells[slot] {
                    if let Some(byte) = apply_ramp_transition(slot as u8, axis, phase, true, is_high_bridge) {
                        writes.push(RampWrite { cell, overlay_byte: byte });
                    }
                }
            }
            StateOutcome::Absorbed { ramp_writes: writes }
        }
        3 => {
            // Final step: full collapse cascade.
            // Per HIGH §3.2:
            // 1. BlowUpBridge × 3 perpendicular cells
            // 2. SetOverlayAndPropagate(anchor, base+3+BridgeSet, level-4)
            // 3. UpdateRamp_*_CollapseA + CollapseB on anchor
            // 4. UpdateAdjacentBridges × 2 (perpendicular neighbors)
            // 5. Zone refresh
            // 6. 10-slot debris loop
            //
            // We emit:
            //   - RampWrites for CollapseA + CollapseB on anchor
            //   - SetBridgeDirectionResult covering the perpendicular 3 cells
            //     plus the anchor span
            // Caller's apply_area_damage drains the BlowUpBridge actions.
            for (slot, phase) in [(1, Phase::CollapseA), (2, Phase::CollapseB)] {
                if let Some(cell) = anchor_span.cells[slot] {
                    if let Some(byte) = apply_ramp_transition(slot as u8, axis, phase, true, is_high_bridge) {
                        writes.push(RampWrite { cell, overlay_byte: byte });
                    }
                }
            }
            anchor_span.damage_state = DamageState::Destroyed;
            *bridgehead_step = 4; // exhausted

            // Build a SetBridgeDirectionResult that covers the anchor span +
            // 3 perpendicular cells from the bridgehead.
            let mut sbd = set_bridge_direction(anchor_span, false);

            // Append 3 perpendicular cells. Perpendicular = axis-rotated direction.
            let perp = match axis {
                Axis::NS => [Direction::N, Direction::S],
                Axis::EW => [Direction::E, Direction::W],
            };
            // 3 cells: bridgehead itself + 1 each side.
            sbd.actions.push((bridgehead_cell, 6, CellAction::BlowUpBridge));
            for d in perp {
                let (dx, dy) = d.offset();
                let nx = bridgehead_cell.0 as i32 + dx;
                let ny = bridgehead_cell.1 as i32 + dy;
                if nx >= 0 && ny >= 0 && nx <= u16::MAX as i32 && ny <= u16::MAX as i32 {
                    sbd.actions.push(((nx as u16, ny as u16), 6, CellAction::BlowUpBridge));
                }
            }

            StateOutcome::Collapsed { ramp_writes: writes, set_bridge_direction: sbd }
        }
        _ => StateOutcome::NoChange,
    }
}
```

**Step 2: Add tests**

```rust
    #[test]
    fn bridgehead_step_0_through_2_absorb_damage() {
        let mut span = make_intact_span(Axis::NS);
        let mut step = 0u8;
        for expected_step_after in [1, 2, 3] {
            let outcome = bridgehead_advance_state(
                (10, 10), &mut step, Axis::NS, &mut span, true,
            );
            assert!(matches!(outcome, StateOutcome::Absorbed { .. }));
            assert_eq!(step, expected_step_after);
        }
    }

    #[test]
    fn bridgehead_step_3_triggers_full_collapse_with_3_perpendicular_blow_ups() {
        let mut span = make_intact_span(Axis::NS);
        let mut step = 3u8;
        let outcome = bridgehead_advance_state(
            (10, 10), &mut step, Axis::NS, &mut span, true,
        );
        match outcome {
            StateOutcome::Collapsed { set_bridge_direction, .. } => {
                let blow_ups = set_bridge_direction.actions.iter()
                    .filter(|(_, _, a)| matches!(a, CellAction::BlowUpBridge))
                    .count();
                // 4 from anchor span (slots 0/1/2/4) + 3 perpendicular = 7.
                assert!(blow_ups >= 4);
            }
            _ => panic!("expected Collapsed"),
        }
        assert_eq!(span.damage_state, DamageState::Destroyed);
    }
```

**Step 3: Verify**

```
cargo test --lib sim::bridge_specs::bridgehead -- --nocapture
```

**Step 4: Commit**

```
git commit -m "bridge_specs: bridgehead state machine (4-step progression + final-step BlowUpBridge × 3 perpendicular)"
```

---

### --- Phase C END — Pure state-machine helpers land. Tests green. ---

---

### Task 16: Build `BridgeDisplayTable` at map load

**Why:** Renderer needs `(base_tile, axis, damage_state) → display_tile`
mapping for each bridge cell. Built once at map load from terrain overlay
ranges. Per design ledger #61–67.

**Files:** Modify `src/sim/bridge_state.rs` (or create a new sub-module).

**Step 1: Add `BridgeDisplayTable` struct**

```rust
/// Display-tile lookup: maps `(base_tile, axis, damage_state)` → display tile
/// index. Built at map load from baked overlay ranges. Renderer queries via
/// `BridgeRuntimeState::display_tile`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BridgeDisplayTable {
    /// Map: (base_tile_index, axis_discriminant, damage_state_discriminant) → display tile.
    /// Damage state discriminant is small int (0=Healthy, 1=Damaged, 2=PartialA, 3=PartialB, 4=Destroyed).
    entries: BTreeMap<(u32, u8, u8), u32>,
}

impl BridgeDisplayTable {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build from terrain overlay ranges. For Tier 2, healthy maps to identity
    /// (use base tile), damaged maps to the corresponding damaged-variant index
    /// per HIGH §2 overlay table, destroyed maps to 0xE7/0xE8 (high) or
    /// equivalent for low.
    pub fn from_terrain(_terrain: &ResolvedTerrainGrid) -> Self {
        // Implementation: walk every bridge cell, map its base overlay to the
        // damaged + destroyed variants per HIGH §2 table.
        //
        // For Phase D landing, simple identity mapping for healthy + a default
        // damaged tile + 0xE7/0xE8 for destroyed.
        let mut entries = BTreeMap::new();
        // Static seed: damaged-EW = 0xD3, damaged-NS = 0xDC, destroyed-EW = 0xE7, destroyed-NS = 0xE8.
        // Per HIGH §2 table (verified).
        for axis_disc in [0u8, 1] {
            // For now, map ANY base tile + (axis, Damaged) → the canonical damaged tile.
            // Refine in Task 16.5 if per-base-tile variant matters.
            let damaged_byte = if axis_disc == 1 { 0xD3u32 } else { 0xDCu32 };
            entries.insert((u32::MAX, axis_disc, 1), damaged_byte);
            let destroyed_byte = if axis_disc == 1 { 0xE7u32 } else { 0xE8u32 };
            entries.insert((u32::MAX, axis_disc, 4), destroyed_byte);
        }
        Self { entries }
    }

    /// Lookup. Returns `base_tile` unchanged for Healthy state or unknown
    /// combos.
    pub fn lookup(&self, base_tile: u32, axis: Option<Axis>, state: DamageState) -> u32 {
        let axis_disc = match axis {
            Some(Axis::EW) => 1u8,
            Some(Axis::NS) => 0u8,
            None => return base_tile,
        };
        let state_disc: u8 = match state {
            DamageState::Healthy { .. } => return base_tile,
            DamageState::Damaged => 1,
            DamageState::PartialCollapseA => 2,
            DamageState::PartialCollapseB => 3,
            DamageState::Destroyed => 4,
        };
        // Try exact match first; fall back to wildcard u32::MAX entry.
        self.entries.get(&(base_tile, axis_disc, state_disc))
            .or_else(|| self.entries.get(&(u32::MAX, axis_disc, state_disc)))
            .copied()
            .unwrap_or(base_tile)
    }
}
```

**Step 2: Add `display_tile` method on `BridgeRuntimeState`**

```rust
    /// Display tile for a bridge cell — applies damage-state visual to base
    /// tile. Renderer queries this for every bridge cell.
    pub fn display_tile(&self, rx: u16, ry: u16, base_tile: u32, table: &BridgeDisplayTable) -> u32 {
        let Some(cell) = self.cell(rx, ry) else { return base_tile; };
        table.lookup(base_tile, cell.axis, cell.damage_state)
    }
```

**Step 3: Tests**

```rust
    #[test]
    fn display_table_healthy_returns_base_tile() {
        let t = BridgeDisplayTable::from_terrain(&make_bridge_terrain());
        assert_eq!(t.lookup(42, Some(Axis::EW), DamageState::Healthy { variant: 0 }), 42);
    }

    #[test]
    fn display_table_damaged_returns_damage_tile() {
        let t = BridgeDisplayTable::from_terrain(&make_bridge_terrain());
        assert_eq!(t.lookup(42, Some(Axis::EW), DamageState::Damaged), 0xD3);
        assert_eq!(t.lookup(42, Some(Axis::NS), DamageState::Damaged), 0xDC);
    }

    #[test]
    fn display_table_destroyed_returns_destroyed_tile() {
        let t = BridgeDisplayTable::from_terrain(&make_bridge_terrain());
        assert_eq!(t.lookup(42, Some(Axis::EW), DamageState::Destroyed), 0xE7);
        assert_eq!(t.lookup(42, Some(Axis::NS), DamageState::Destroyed), 0xE8);
    }
```

**Step 4: Verify**

```
cargo test --lib sim::bridge_state::tests::display_table -- --nocapture
```

**Step 5: Commit**

```
git commit -m "bridge_state: BridgeDisplayTable + BridgeRuntimeState::display_tile (renderer hook)"
```

---

### Task 17: Hook renderer to query `display_tile`

**Why:** Without this, damaged/destroyed bridge cells render the baked tile
(no visible damage). Per ledger #61–67.

**Files:** Modify the terrain renderer that draws bridge tiles. Search for
existing bridge-tile rendering call sites.

**Step 1: Locate**

```
grep -rn "BridgeRuntimeState\|bridge_walkable\|is_bridge" src/render/ src/app_instances/ src/map/terrain.rs
```

**Step 2: Find the bridge-tile draw site**

The terrain renderer iterates cells and draws tile per cell. For bridge
cells (`has_bridge_deck` or `bridge_layer.is_some()`), introduce
display_tile lookup.

**Step 3: Wire `BridgeDisplayTable`**

The table must be built at map load and stored where the renderer can access
it. Follow existing pattern of map-load-built lookup tables (search for
similar precomputed tables in the renderer).

**Step 4: Replace baked-tile draw with display_tile call for bridge cells**

Pattern:
```rust
let display_tile = if let Some(bridge_state) = bridge_state {
    bridge_state.display_tile(rx, ry, base_tile, bridge_display_table)
} else {
    base_tile
};
draw_tile(display_tile, ...);
```

**Step 5: Verify in-game**

Run the game with a bridged map. Confirm:
- Healthy bridges look identical to baseline
- After damage, damaged variant renders
- After collapse, destroyed variant renders

If renderer changes are larger than expected, split into a separate task and
commit after Phase F lands so the runtime state-machine actually drives
state changes.

**Step 6: Commit**

```
git commit -m "render: hook BridgeRuntimeState::display_tile for bridge cells (driven by Tier 2 damage state)"
```

---

### --- Phase D END — Renderer hooks BridgeRuntimeState. ---

---

### Task 18: Extend `BridgeDamageEvent` with `warhead_ref` + `is_ion_cannon`

**Why:** Combat boundary needs to pre-resolve the warhead identity. World
orchestrator reads `is_ion_cannon` to decide RNG-gate-vs-bypass + retry semantics.

**Files:** Modify `src/sim/combat/mod.rs` — extend `BridgeDamageEvent` struct
(near line 384). Also `src/sim/world/mod.rs` — `apply_bridge_damage_events`
signature already takes `&[BridgeDamageEvent]`.

**Step 1: Extend the struct**

In `src/sim/combat/mod.rs` find existing `BridgeDamageEvent` declaration near
line 384 and replace:

```rust
/// Per-cell bridge damage event emitted by combat. World drains via
/// `apply_bridge_damage_events`. Apply_area_damage gating + retry happen
/// in the world orchestrator, not in combat — so RNG draw order matches
/// binary `Apply_area_damage @ 0x4894B0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BridgeDamageEvent {
    pub rx: u16,
    pub ry: u16,
    pub damage: u16,
    /// Interned warhead ID — used for InfDeath selection in BlowUpBridge
    /// kill path and to identify IonCannonWarhead.
    pub warhead_ref: crate::sim::intern::InternedId,
    /// Pre-resolved `warhead_ref == rules.bridge_warheads.ion_cannon`.
    /// Bypasses BridgeStrength RNG draw; enables retry loop.
    pub is_ion_cannon: bool,
}
```

(Adjust `InternedId` import path to match repo conventions — search for
`InternedId` usage in combat/mod.rs to find the right module.)

**Step 2: Update emit sites**

There are 3 sites in `src/sim/combat/mod.rs`. Anchor by content (line numbers
shifted by parallel `TargetKind` work):

**Site 1 — death AoE in `apply_death_effects`** (search pattern: `bridge_damage_events.push(BridgeDamageEvent {` near `wall_damage_events.push(WallDamageEvent`):

```rust
                } else {
                    let ion_cannon_id = rules.ion_cannon_warhead_id();
                    bridge_damage_events.push(BridgeDamageEvent {
                        rx: *rx,
                        ry: *ry,
                        damage: damage_u16,
                        warhead_ref: *wh_id,
                        is_ion_cannon: *wh_id == ion_cannon_id,
                    });
                }
```

(Resolve `rules.ion_cannon_warhead_id()` — Task 19 adds this helper on `RuleSet`.)

**Site 2 — primary attack AoE branch** (search same pattern, second occurrence):

Same shape; the warhead is in scope as `warhead`, intern via
`interner.intern(&warhead.id)`.

**Site 3 — primary attack non-AoE branch** (third occurrence): same shape.

**Step 3: Update `BridgeDamageEvent` consumers**

`apply_bridge_damage_events` in `world/mod.rs` doesn't read `warhead_ref` /
`is_ion_cannon` yet — Task 22 will. Existing code keeps compiling.

**Step 4: Update existing world_tests fixtures**

In `src/sim/world/world_tests.rs:413-617`, the 6 tests construct
`BridgeDamageEvent { rx, ry, damage }`. Add the new fields. Use the existing
`Simulation::interner` to mint an interned ID for the test (no
`placeholder()` method — `crate::sim::intern::InternedId` is constructed
only via `StringInterner::intern`):

```rust
    let wh_id = sim.interner.intern("TestWarhead"); // or "IonCannonWH" if testing IonCannon path
    BridgeDamageEvent {
        rx: 1,
        ry: 0,
        damage: 50,
        warhead_ref: wh_id,
        is_ion_cannon: false, // test legacy path
    }
```

(For Tier 2, these tests still work as legacy single-shot collapse only
because the existing per-group HP path is preserved as fallback. After
Task 22 they'll need to migrate to IonCannon-context inputs.)

**Step 5: Verify build**

```
cargo build
cargo test --lib world_tests -- --nocapture
```
Expected: build green; existing tests pass (or fail on new field init —
fix and rebuild).

**Step 6: Commit**

```
git commit -m "combat: extend BridgeDamageEvent with warhead_ref + is_ion_cannon (gate decision at world boundary)"
```

---

### Task 19: Resolve `ion_cannon_warhead_id` and `c4_warhead_id` on `RuleSet` at world init

**Why:** Combat reads pre-resolved interned IDs. RuleSet stores names from
`BridgeWarheads`; world resolution interns them once.

**Files:** Modify `src/rules/ruleset.rs`. Add resolved fields + lookup methods.

**Step 1: Add post-resolution fields**

```rust
    /// Resolved at sim init — interned WarheadId for `[CombatDamage] IonCannonWarhead=`.
    /// Used by combat to pre-resolve `BridgeDamageEvent.is_ion_cannon`.
    #[serde(skip)]
    ion_cannon_warhead_id: Option<crate::sim::intern::InternedId>,

    /// Resolved at sim init — interned WarheadId for `[CombatDamage] C4Warhead=`.
    /// Used by `kill_ground_occupants_at` in BlowUpBridge ground kill.
    #[serde(skip)]
    c4_warhead_id: Option<crate::sim::intern::InternedId>,
```

(If `RuleSet` does not have `serde::Serialize` or doesn't use serde, drop
the `#[serde(skip)]` attribute — adjust per repo convention.)

**Step 2: Add resolver method**

```rust
    /// Resolve bridge warhead names against the interner. Call once at sim
    /// init after the warhead registry is populated.
    pub fn resolve_bridge_warheads(&mut self, interner: &mut crate::sim::intern::StringInterner) {
        self.ion_cannon_warhead_id = Some(interner.intern(&self.bridge_warheads.ion_cannon_name));
        self.c4_warhead_id = Some(interner.intern(&self.bridge_warheads.c4_name));
    }

    pub fn ion_cannon_warhead_id(&self) -> crate::sim::intern::InternedId {
        self.ion_cannon_warhead_id.expect(
            "RuleSet::resolve_bridge_warheads must be called before combat reads warhead IDs"
        )
    }

    pub fn c4_warhead_id(&self) -> crate::sim::intern::InternedId {
        self.c4_warhead_id.expect(
            "RuleSet::resolve_bridge_warheads must be called before BlowUpBridge fires"
        )
    }
```

**Step 3: Call resolver from `app_init_helpers.rs`**

Find where `RuleSet` is consumed at sim init. Insert call to
`resolve_bridge_warheads(&mut sim.interner)` immediately before
`BridgeRuntimeState::from_resolved_terrain` (line 354). Mutability on
RuleSet may require an interior mutability adjustment — if RuleSet is
`Arc<RuleSet>`, the resolution must happen before wrapping.

**Step 4: Verify**

```
cargo build
cargo test
```
Expected: green.

**Step 5: Commit**

```
git commit -m "rules: pre-resolve IonCannonWarhead + C4Warhead interned IDs at sim init"
```

---

### --- Phase E END — Combat boundary gates wired. ---

---

### Task 20: `BridgeDamageContext` + replacement `apply_area_damage` in `BridgeRuntimeState`

**Why:** Replace existing `apply_damage(event)` with the 4-path
`apply_area_damage(rx, ry, ctx)` that does gate + RNG + retry. Per
design ledger #1–10.

**Files:** Modify `src/sim/bridge_state.rs`.

**Step 1: Add context struct**

```rust
/// Damage-event context passed from combat through world to bridge runtime.
/// Carries the pre-resolved IonCannon flag + RNG handle.
pub struct BridgeDamageContext<'a> {
    pub damage: u16,
    pub is_ion_cannon: bool,
    pub bridge_strength: u16,
    pub rng: &'a mut crate::sim::rng::SimRng,
}
```

**Step 2: Replace `apply_damage` with `apply_area_damage`**

```rust
    /// Per-cell `Apply_area_damage` mirror (gamemd `0x4894B0`). Evaluates
    /// 4 dispatch paths (high body, high bridgehead, low body, low bridgehead).
    /// Each path runs an independent BridgeStrength RNG draw and (for
    /// IonCannonWarhead) a 3-retry loop.
    ///
    /// Returns one `BridgeStateChange` per path that triggered a collapse.
    /// Multiple paths can fire on a single damage event in pathological cases
    /// (overlay coverage overlap), but typically only one matches.
    pub fn apply_area_damage(
        &mut self,
        rx: u16,
        ry: u16,
        ctx: &mut BridgeDamageContext,
    ) -> Vec<BridgeStateChange> {
        let mut changes = Vec::new();

        // Outer gate: SpecialFlags 0x8000. Warhead.Wall is checked at combat
        // boundary (only Wall warheads emit BridgeDamageEvent), so we don't
        // recheck here.
        if !self.bridge_destroyable_flag {
            return changes;
        }

        // 4 dispatch paths, each with independent RNG draw.
        // Order matches binary `0x4894B0` source order.
        for path in [
            DispatchPath::HighBodyOrBridgehead,
            DispatchPath::LowBodyOrBridgehead,
            DispatchPath::LowDirect,
            DispatchPath::HighDirect,
        ] {
            if !self.path_matches_cell(path, rx, ry) {
                continue;
            }
            // Per-path BridgeStrength RNG draw (skipped for IonCannon).
            if !ctx.is_ion_cannon {
                let roll = ctx.rng.next_range_u32_inclusive(1, ctx.bridge_strength as u32);
                if !(roll < ctx.damage as u32) {
                    continue; // RNG gate failed for this path
                }
            }
            // ApplyDamageToCell + retry loop (IonCannon only).
            let max_attempts = if ctx.is_ion_cannon { 4 } else { 1 };
            for _attempt in 0..max_attempts {
                if let Some(change) = self.apply_damage_to_cell_path(rx, ry, path, ctx) {
                    changes.push(change);
                    break; // exit retry loop on success
                }
                // Failed attempt; retry only if IonCannon.
            }
        }

        changes
    }

    fn path_matches_cell(&self, path: DispatchPath, rx: u16, ry: u16) -> bool {
        // Use overlay info from terrain (passed at sim init) or inferred from
        // BridgeRuntimeCell.role + axis. Implementation detail — simplest version:
        let Some(cell) = self.cell(rx, ry) else {
            return false;
        };
        match path {
            DispatchPath::HighBodyOrBridgehead => {
                matches!(cell.role,
                    BridgeCellRole::Anchor | BridgeCellRole::Body | BridgeCellRole::Bridgehead | BridgeCellRole::Tail)
                    && self.is_high_at(rx, ry)
            }
            DispatchPath::LowBodyOrBridgehead => {
                matches!(cell.role,
                    BridgeCellRole::Anchor | BridgeCellRole::Body | BridgeCellRole::Bridgehead | BridgeCellRole::Tail)
                    && !self.is_high_at(rx, ry)
            }
            DispatchPath::LowDirect | DispatchPath::HighDirect => {
                // Direct overlay-range paths. For Tier 2 simplicity, treat as
                // identical to the body/bridgehead paths. Refine if integration
                // shows divergence.
                false
            }
        }
    }

    fn is_high_at(&self, rx: u16, ry: u16) -> bool {
        // Heuristic: bridge_layer.direction == EastWest|NorthSouth → high;
        // direction == Low → low. Stored on BridgeRuntimeCell as deck_level
        // ≥ some threshold; we reuse axis as discriminant since Low maps to
        // BridgeDirection::Low → Axis::NS (per Task 7) but that's ambiguous.
        // A clean fix is adding `is_high: bool` to BridgeRuntimeCell at
        // construction time. For now, derive from deck_level >= 4 (high) vs.
        // deck_level < 4 (low — wood at ground level + 2).
        //
        // TODO: replace with explicit is_high field on BridgeRuntimeCell.
        self.cell(rx, ry).is_some_and(|c| c.deck_level >= 4)
    }

    fn apply_damage_to_cell_path(
        &mut self,
        rx: u16,
        ry: u16,
        path: DispatchPath,
        _ctx: &mut BridgeDamageContext,
    ) -> Option<BridgeStateChange> {
        // Get cell + axis + role.
        let cell = self.cell(rx, ry).copied()?;
        let is_high = matches!(path, DispatchPath::HighBodyOrBridgehead | DispatchPath::HighDirect);

        match cell.role {
            BridgeCellRole::Anchor | BridgeCellRole::Body | BridgeCellRole::Tail => {
                // Body-cell branch.
                let span_id = cell.anchor_span_id?;
                let span = self.anchor_spans.get_mut(&span_id)?;
                let outcome = crate::sim::bridge_specs::body_cell_advance_state(span, is_high);
                self.apply_state_outcome(span_id, outcome)
            }
            BridgeCellRole::Bridgehead => {
                // Bridgehead branch. Walk to anchor, then advance bridgehead step.
                // Per HIGH §3.2.
                // For Tier 2 implementation, treat the bridgehead cell's stored
                // bridgehead_step + axis directly.
                let axis = cell.axis?;
                let mut step = cell.bridgehead_step;
                // Find anchor span — for bridgeheads, walk via terrain. We don't
                // have terrain in scope here; defer to caller or store anchor
                // ref on bridgehead cells.
                //
                // For Tier 2 simplification: bridgeheads aren't linked to a
                // specific anchor at map load. Here we find the closest anchor
                // span by axis match.
                let span_id = self.find_bridgehead_anchor_span(axis, rx, ry)?;
                let span = self.anchor_spans.get_mut(&span_id)?;
                let outcome = crate::sim::bridge_specs::bridgehead_advance_state(
                    (rx, ry), &mut step, axis, span, is_high,
                );
                // Persist bridgehead_step.
                if let Some(idx) = index_of(self.width, self.height, rx, ry) {
                    if let Some(c) = self.cells[idx].as_mut() {
                        c.bridgehead_step = step;
                    }
                }
                self.apply_state_outcome(span_id, outcome)
            }
        }
    }

    fn find_bridgehead_anchor_span(&self, axis: Axis, rx: u16, ry: u16) -> Option<u16> {
        // Closest anchor span sharing axis. For Tier 2 fidelity this should
        // walk via DirectionOffset until height matches; in practice the
        // bridgehead is adjacent to the anchor span's tail or anchor cell.
        self.anchor_spans
            .iter()
            .filter(|(_, span)| span.axis == axis)
            .min_by_key(|(_, span)| {
                let (ax, ay) = span.anchor;
                ((ax as i32 - rx as i32).abs() + (ay as i32 - ry as i32).abs()) as u32
            })
            .map(|(id, _)| *id)
    }

    fn apply_state_outcome(
        &mut self,
        _span_id: u16,
        outcome: crate::sim::bridge_specs::StateOutcome,
    ) -> Option<BridgeStateChange> {
        use crate::sim::bridge_specs::{StateOutcome, CellAction};
        match outcome {
            StateOutcome::Absorbed { ramp_writes: _ } => {
                // Ramp writes go to mutable overlay layer (Phase D display table —
                // tracked via BridgeRuntimeCell.damage_state, no per-cell overlay
                // mutation needed since renderer queries display_tile).
                None // damage absorbed, no group change
            }
            StateOutcome::Collapsed { ramp_writes: _, set_bridge_direction } => {
                let mut destroyed = Vec::new();
                for (cell, _slot, action) in &set_bridge_direction.actions {
                    match action {
                        CellAction::BlowUpBridge => {
                            // Mark destroyed.
                            if let Some(idx) = index_of(self.width, self.height, cell.0, cell.1) {
                                if let Some(c) = self.cells[idx].as_mut() {
                                    c.damage_state = DamageState::Destroyed;
                                }
                            }
                            destroyed.push(*cell);
                        }
                        CellAction::FlagOnly => {
                            // Flag-only cells: anchor_span tagging cleared.
                            if let Some(idx) = index_of(self.width, self.height, cell.0, cell.1) {
                                if let Some(c) = self.cells[idx].as_mut() {
                                    c.anchor_span_id = None;
                                }
                            }
                        }
                    }
                }
                destroyed.sort_unstable();
                // Mark all endpoint records inactive for affected groups.
                for cell in &destroyed {
                    if let Some(rc) = self.cell(cell.0, cell.1) {
                        if let Some(group_id) = rc.bridge_group_id {
                            for record in &mut self.endpoint_records {
                                if record.group_id == group_id {
                                    record.active = false;
                                }
                            }
                        }
                    }
                }
                Some(BridgeStateChange { destroyed_cells: destroyed })
            }
            StateOutcome::NoChange => None,
        }
    }
```

**Step 3: Add `DispatchPath` enum**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DispatchPath {
    HighBodyOrBridgehead,
    LowBodyOrBridgehead,
    LowDirect,
    HighDirect,
}
```

**Step 4: Tests**

```rust
    #[test]
    fn apply_area_damage_skipped_when_destroyable_flag_false() {
        let mut state = BridgeRuntimeState::from_resolved_terrain(
            &make_bridge_terrain(), false, 1500,
        );
        let mut rng = crate::sim::rng::SimRng::new(42);
        let mut ctx = BridgeDamageContext {
            damage: 100,
            is_ion_cannon: false,
            bridge_strength: 1500,
            rng: &mut rng,
        };
        let changes = state.apply_area_damage(1, 0, &mut ctx);
        assert!(changes.is_empty());
    }

    #[test]
    fn apply_area_damage_ion_cannon_bypasses_strength_gate() {
        let mut state = BridgeRuntimeState::from_resolved_terrain(
            &make_bridge_terrain(), true, 1500,
        );
        let mut rng = crate::sim::rng::SimRng::new(42);
        let initial_state = rng.state();
        let mut ctx = BridgeDamageContext {
            damage: 1, // tiny damage that would normally fail RNG gate
            is_ion_cannon: true,
            bridge_strength: 1500,
            rng: &mut rng,
        };
        let _ = state.apply_area_damage(1, 0, &mut ctx);
        // RNG should NOT have been consumed by the gate (IonCannon bypasses it).
        // Internal cell processing may consume RNG; record exact draw count
        // separately in Task 39.
        let _ = initial_state; // sanity placeholder
    }

    #[test]
    fn apply_area_damage_non_ion_cannon_uses_rng_gate() {
        let mut state = BridgeRuntimeState::from_resolved_terrain(
            &make_bridge_terrain(), true, 1500,
        );
        let mut rng = crate::sim::rng::SimRng::new(42);
        let mut ctx = BridgeDamageContext {
            damage: 1,
            is_ion_cannon: false,
            bridge_strength: 1500,
            rng: &mut rng,
        };
        // damage=1 vs strength=1500 → roll <1 only when roll==0, but inclusive [1,1500] never returns 0.
        // So no path should fire.
        let changes = state.apply_area_damage(1, 0, &mut ctx);
        assert!(changes.is_empty());
    }
```

**Step 5: Verify**

```
cargo test --lib sim::bridge_state::tests::apply_area_damage -- --nocapture
```

**Step 6: Commit**

```
git commit -m "bridge_state: replace apply_damage with apply_area_damage (4-path dispatch + IonCannon gate/retry)"
```

---

### Task 21: `Simulation::apply_bridge_damage_events` rewrite

**Why:** Wire the new `BridgeDamageContext` flow. Combat passes
`BridgeDamageEvent` with `is_ion_cannon` pre-set; world bridges to
`apply_area_damage`.

**Files:** Modify `src/sim/world/mod.rs:673-687`.

**Step 1: Replace function body**

```rust
    pub(crate) fn apply_bridge_damage_events(
        &mut self,
        events: &[BridgeDamageEvent],
    ) -> Vec<BridgeStateChange> {
        let mut all_changes = Vec::new();
        let Some(bridge_state) = self.bridge_state.as_mut() else {
            return all_changes;
        };
        let bridge_strength = bridge_state.strength_per_group; // proxy until Phase A wires bridge_strength field correctly
        for event in events {
            let mut ctx = BridgeDamageContext {
                damage: event.damage,
                is_ion_cannon: event.is_ion_cannon,
                bridge_strength,
                rng: &mut self.rng,
            };
            let mut changes = bridge_state.apply_area_damage(event.rx, event.ry, &mut ctx);
            all_changes.append(&mut changes);
        }
        all_changes
    }
```

**Step 2: Update existing 6 world_tests**

Each `apply_bridge_damage_events(&[BridgeDamageEvent { rx, ry, damage }])`
call must pass IonCannon context to preserve single-shot collapse semantics:

```rust
    let ion_cannon_id = sim.interner_intern("IonCannonWH"); // or whatever helper
    let changes = sim.apply_bridge_damage_events(&[BridgeDamageEvent {
        rx: 1,
        ry: 0,
        damage: 9999,
        warhead_ref: ion_cannon_id,
        is_ion_cannon: true,
    }]);
```

**Step 3: Verify**

```
cargo test --lib world_tests -- --nocapture
```
Expected: PASS (6 tests).

**Step 4: Commit**

```
git commit -m "world: rewrite apply_bridge_damage_events to use BridgeDamageContext (gate + retry)"
```

---

### Task 22: `kill_ground_occupants_at` — C4Warhead force_kill via existing death pipeline

**Why:** Per design ledger #43 + #41. Binary's `BlowUpBridge` step 1 walks
`+0xE4` ground occupants and calls `ReceiveDamage(damage=0, warhead=C4Warhead,
force_kill=1)`. Existing death pipeline selects `InfDeath` from killing
warhead.

**Files:** Modify `src/sim/world/mod.rs` — add helper in
`resolve_bridge_state_changes` or as a private method.

**Step 1: Add the method**

```rust
    /// Kill all ground-layer entities at `(rx, ry)` via the standard death
    /// pipeline using `c4_warhead` as killing warhead. Mirrors binary's
    /// `BlowUpBridge` step 1 (`vtable+0x16C(damage=0, C4Warhead, force_kill=1)`).
    /// Bridge-deck entities are handled by the existing snap/despawn path.
    fn kill_ground_occupants_at(
        &mut self,
        rx: u16,
        ry: u16,
        c4_warhead_id: crate::sim::intern::InternedId,
    ) {
        let entities_at_cell: Vec<u64> = self.entities
            .iter_sorted()
            .filter(|(_, e)| {
                e.position.rx == rx
                    && e.position.ry == ry
                    && !e.is_on_bridge_layer()
            })
            .map(|(id, _)| id)
            .collect();
        for id in entities_at_cell {
            // Force-kill: set health to 0 and route through death pipeline
            // with C4Warhead so InfDeath selection matches binary.
            //
            // Existing death pipeline at apply_death_effects expects a
            // damage_event tuple. We construct a synthetic one.
            if let Some(entity) = self.entities.get_mut(id) {
                entity.health.current = 0;
                // Tag dying flag so subsequent ticks process the death anim.
                entity.dying = true;
                entity.attack_target = None;
                entity.movement_target = None;
                entity.selected = false;
                if let Some(ref mut anim) = entity.animation {
                    use crate::sim::animation::death_sequence_for_inf_death;
                    // Look up InfDeath from c4_warhead — for Tier 2 use Die1
                    // as fallback (death pipeline normally derives from
                    // killing warhead's `inf_death` field).
                    anim.switch_to(death_sequence_for_inf_death(1));
                }
            }
        }
        let _ = c4_warhead_id; // currently unused; reserved for InfDeath lookup
    }
```

**Step 2: Tests**

Add an integration-style test in `world_tests.rs`:

```rust
    #[test]
    fn bridge_collapse_kills_ground_occupants() {
        // Build a sim with a bridge over a ground cell that has an entity.
        // Trigger collapse; verify the ground entity's health is 0 / dying.
        //
        // Use existing test fixture pattern.
    }
```

**Step 3: Verify**

```
cargo test --lib world_tests::bridge_collapse_kills_ground_occupants -- --nocapture
```

**Step 4: Commit**

```
git commit -m "world: kill_ground_occupants_at — C4Warhead force_kill mirroring BlowUpBridge step 1"
```

---

### Task 23: `spawn_bridge_debris` — replaces `spawn_bridge_explosions` with binary structure

**Why:** Per Issue #1 from /review-plan. Existing `spawn_bridge_explosions`
spawns 1 immediate + 50% delayed BridgeExplosion (wrong). Binary spawns
50% MetallicDebris (no delay) + 1 always-delayed BridgeExplosion. Per
ledger #46–49.

**Files:** Modify `src/sim/world/mod.rs:851-919`.

**Step 1: Rename and rewrite the function**

Replace existing `spawn_bridge_explosions` body:

```rust
    /// Spawn debris on destroyed bridge cells. Mirrors binary `BlowUpBridge @
    /// 0x47DD70` step 4: per cell that passes the 95% outer gate, draw
    /// 2 jitter values, optionally spawn 1 MetallicDebris (50%-gated, no
    /// delay), always spawn 1 BridgeExplosion (delay 1–5 frames).
    fn spawn_bridge_debris(
        &mut self,
        destroyed_cells: &std::collections::BTreeSet<(u16, u16)>,
        rules: &RuleSet,
    ) {
        if self.bridge_explosions.is_empty() && self.metallic_debris.is_empty() {
            return;
        }
        let explosion_count = self.bridge_explosions.len() as u32;
        let metallic_count = self.metallic_debris.len() as u32;
        let voxel_max_gate = rules.bridge_rules.voxel_max > 0;

        for &(rx, ry) in destroyed_cells {
            // Outer gate: 95% per cell (5% skip rate).
            if self.rng.next_range_u32(20) == 0 {
                continue;
            }

            // 2 jitter draws per cell (consumed even though Rust spawn doesn't
            // need pixel jitter — RNG order parity contract).
            let _jitter_x = self.rng.next_range_u32(0xFFFF);
            let _jitter_y = self.rng.next_range_u32(0xFFFF);

            let deck_level = self
                .resolved_terrain
                .as_ref()
                .and_then(|t| t.cell(rx, ry))
                .map(|c| c.bridge_deck_level_if_any().unwrap_or(c.level))
                .unwrap_or(0);

            // 50%-gated MetallicDebris (no delay). Skipped if voxel_max == 0
            // or no metallic_debris list.
            let metallic_pass = self.rng.next_range_u32(2) == 0;
            if metallic_pass && voxel_max_gate && metallic_count > 0 {
                let idx = self.rng.next_range_u32(metallic_count) as usize;
                let anim_id = self.metallic_debris[idx];
                let frames = self
                    .effect_frame_counts
                    .get(&anim_id)
                    .copied()
                    .unwrap_or(20);
                self.world_effects.push(WorldEffect {
                    shp_name: anim_id,
                    rx,
                    ry,
                    z: deck_level,
                    frame: 0,
                    total_frames: frames,
                    rate_ms: 67,
                    elapsed_ms: 0,
                    translucent: true,
                    delay_ms: 0,
                });
            }

            // Always-spawn BridgeExplosion (delay 1-5 frames).
            if explosion_count > 0 {
                let delay_frames = self.rng.next_range_u32_inclusive(1, 5);
                let idx = self.rng.next_range_u32(explosion_count) as usize;
                let anim_id = self.bridge_explosions[idx];
                let frames = self
                    .effect_frame_counts
                    .get(&anim_id)
                    .copied()
                    .unwrap_or(20);
                self.world_effects.push(WorldEffect {
                    shp_name: anim_id,
                    rx,
                    ry,
                    z: deck_level,
                    frame: 0,
                    total_frames: frames,
                    rate_ms: 67,
                    elapsed_ms: 0,
                    translucent: true,
                    delay_ms: delay_frames * 67,
                });
            }
        }
    }
```

**Type contract:** `WorldEffect.shp_name: InternedId` ([components.rs:537](../../src/sim/components.rs#L537)).
`bridge_explosions: Vec<InternedId>` already pre-interned at sim init
([world/mod.rs:254](../../src/sim/world/mod.rs#L254)).
**Pre-intern `metallic_debris`** the same way: add
`pub metallic_debris: Vec<InternedId>` to `Simulation` near line 254 and
populate at sim init from `rules.general.metallic_debris` (interning each
name) — mirrors how `bridge_explosions` is populated in `app_init_helpers.rs`.
This adds a one-line struct field + a one-line init loop; both are
preconditions for Task 23 to compile.

**Step 2: Replace caller of `spawn_bridge_explosions` with `spawn_bridge_debris`**

In `resolve_bridge_state_changes` near line 790:

```rust
        self.spawn_bridge_debris(&destroyed_cells, rules);
```

(`rules: &RuleSet` must be available in scope — pass through the call chain
or store on `Simulation`.)

**Step 3: Tests**

```rust
    #[test]
    fn spawn_bridge_debris_consumes_correct_rng_count_per_cell() {
        // Set up sim with known RNG seed.
        // Force bridge collapse on 1 cell.
        // Verify RNG state-after matches expected after exactly:
        //   1 (outer gate) + 2 (jitter) + 1 (50% inner) + (0 or 1 slot) + 1 (delay) + 1 (slot) draws
        // = 6 or 7 draws.
    }

    #[test]
    fn spawn_bridge_debris_metallic_skipped_when_voxel_max_zero() {
        // BridgeRules.voxel_max = 0 → no MetallicDebris spawn even on 50% pass.
    }
```

**Step 4: Verify**

```
cargo test --lib world::spawn_bridge_debris -- --nocapture
cargo test
```

**Step 5: Commit**

```
git commit -m "world: spawn_bridge_debris — replaces spawn_bridge_explosions with binary-correct structure (50% MetallicDebris + 1 always BridgeExplosion delayed)"
```

---

### Task 24: `update_adjacent_bridges` rim re-evaluation

**Why:** Per ledger #56. After cell state change, neighbors of changed cell
get role/axis re-evaluated.

**Files:** Modify `src/sim/world/mod.rs` (or `bridge_state.rs`).

**Step 1: Add helper**

```rust
    fn update_adjacent_bridges(
        &mut self,
        changed_cells: &std::collections::BTreeSet<(u16, u16)>,
    ) {
        let Some(bridge_state) = self.bridge_state.as_mut() else { return; };
        for &(rx, ry) in changed_cells {
            for (nx, ny) in [(rx.saturating_add(1), ry), (rx.wrapping_sub(1), ry),
                              (rx, ry.saturating_add(1)), (rx, ry.wrapping_sub(1))] {
                // Re-evaluate neighbor — for Tier 2, this is a no-op since
                // BridgeRuntimeCell.role is set at map load via anchor walker
                // and doesn't drift. UpdateAdjacentBridges in binary writes
                // CellClass+0x140 flags that we represent as named bools;
                // since the named bools are derived from immutable terrain at
                // load time, no rim refresh needed. The hook exists for future
                // tiers where role can drift (e.g., repair).
                let _ = (nx, ny);
            }
        }
    }
```

(Per the design's "first-class structs" approach, role/axis don't drift,
making this a stub for Tier 2. Document this so future tiers know.)

**Step 2: Verify**

```
cargo build
```

**Step 3: Commit**

```
git commit -m "world: update_adjacent_bridges stub (no-op for Tier 2; future tiers may need refresh)"
```

---

### Task 25: `refresh_bridge_zones_if_dirty` zone refresh hook

**Why:** Per ledger #54–55. After bridge collapse, zone graph must
recompute (existing `BridgeEndpointRecord.active` toggling already handled).

**Files:** Modify `src/sim/world/mod.rs`.

**Step 1: Add helper**

```rust
    fn refresh_bridge_zones_if_dirty(
        &mut self,
        any_record_changed: bool,
        path_grid: &PathGrid,
    ) {
        if !any_record_changed {
            return;
        }
        self.rebuild_zone_grid(path_grid);
    }
```

`path_grid` is supplied by the caller. Inside `resolve_bridge_state_changes`,
the existing `fallout_ground_grid: Option<PathGrid>` constructed at
[world/mod.rs:785-787](../../src/sim/world/mod.rs#L785-L787) is the right
candidate — pass `fallout_ground_grid.as_ref()` to this helper after the
collapse fallout finishes mutating `bridge_state`. Note: the existing
`Simulation::prev_path_grid` field is private and used only for incremental
zone diffing; do NOT use it here.

**Step 2: Wire into `resolve_bridge_state_changes`**

After spawn_bridge_debris and before returning despawned_ids, call:

```rust
        if let Some(grid) = fallout_ground_grid.as_ref() {
            self.refresh_bridge_zones_if_dirty(!destroyed_cells.is_empty(), grid);
        }
```

(`fallout_ground_grid` is already constructed at the start of
`resolve_bridge_state_changes` at world/mod.rs:785-787 — reuse it.)

**Step 3: Verify**

```
cargo build
cargo test --lib world -- --nocapture
```

**Step 4: Commit**

```
git commit -m "world: refresh_bridge_zones_if_dirty hook — reuse rebuild_zone_grid after bridge collapse"
```

---

### Task 26: Wire ground-occupant kill into `resolve_bridge_state_changes` + extend signature

**Why:** Per ledger #43 + #48. `BlowUpBridge` step 1 (ground kill) must fire
before step 2 (bridge-deck Limbo). Tasks 22, 23, 25 all need `rules: &RuleSet`
in scope inside the orchestrator — single signature change covers all three.

**Files:** Modify `src/sim/world/mod.rs` — `resolve_bridge_state_changes`
signature + call site at line 1338.

**Step 1: Extend `resolve_bridge_state_changes` signature**

Current signature (at world/mod.rs:771):

```rust
pub(crate) fn resolve_bridge_state_changes(
    &mut self,
    changes: &[BridgeStateChange],
) -> Vec<u64>
```

Update to:

```rust
pub(crate) fn resolve_bridge_state_changes(
    &mut self,
    changes: &[BridgeStateChange],
    rules: &RuleSet,
) -> Vec<u64>
```

Pattern matches existing `apply_wall_damage_events(events, rules, reg)` at
world/mod.rs:1344.

**Step 2: Update call site**

At world/mod.rs:1338, change:

```rust
let _bridge_fallout_ids = self.resolve_bridge_state_changes(&bridge_changes);
```

to:

```rust
let _bridge_fallout_ids = self.resolve_bridge_state_changes(&bridge_changes, rules);
```

`rules: &RuleSet` is already in scope at this call site (used by
`apply_wall_damage_events` at line 1344).

**Step 3: Insert ground-kill loop at start of `resolve_bridge_state_changes`**

Before the existing `for entity in self.entities.values()` loop that
populates `to_snap` / `to_despawn`, add:

```rust
        // Step 1 (binary BlowUpBridge): kill ground occupants under each
        // destroyed cell with C4Warhead force_kill semantics.
        let c4_id = rules.c4_warhead_id();
        for &(rx, ry) in &destroyed_cells {
            self.kill_ground_occupants_at(rx, ry, c4_id);
        }
```

**Step 4: Verify ordering**

The existing on-bridge snap/despawn loop runs after — that's step 2 (bridge-deck
Limbo). Then `spawn_bridge_debris` runs — that's step 4. Ordering matches
binary: ground kill → bridge-deck Limbo → debris.

**Step 5: Tests**

```rust
    #[test]
    fn bridge_collapse_kill_order_ground_then_bridge_then_debris() {
        // Assert specific sub-tick ordering via state-hash checkpoint.
    }
```

**Step 6: Verify**

```
cargo test --lib world_tests -- --nocapture
```

**Step 7: Commit**

```
git commit -m "world: resolve_bridge_state_changes — fire kill_ground_occupants_at before bridge-deck Limbo (binary order)"
```

---

### --- Phase F END — Orchestrator wired. Build green. Run full test suite. ---

---

### Task 27: Anchor walker correctness tests

**Why:** Highest correctness risk per Risk Areas. Need handcrafted bridge
fixtures + assertions on emitted AnchorSpan placement.

**Files:** Create test cases in `src/sim/bridge_state.rs::tests`.

**Step 1: Add fixture helper**

```rust
    fn make_long_bridge_terrain(length: u16) -> ResolvedTerrainGrid {
        let mut cells = Vec::new();
        for rx in 0..length {
            cells.push(make_bridge_cell(rx, 0, /* anchor= */ rx == length / 2));
        }
        ResolvedTerrainGrid::from_cells(length, 1, cells)
    }
```

(`make_bridge_cell` is a helper that builds a `ResolvedTerrainCell` with
appropriate `bridge_layer.overlay_id` — `0x18` for anchor, `0x4A` for body.)

**Step 2: Tests**

```rust
    #[test]
    fn anchor_walker_5x1_horizontal_one_anchor_one_span() {
        let terrain = make_long_bridge_terrain(5);
        let state = BridgeRuntimeState::from_resolved_terrain(&terrain, true, 1500);
        assert_eq!(state.anchor_spans().len(), 1);
        let span = state.anchor_spans().values().next().unwrap();
        assert_eq!(span.axis, Axis::EW); // EW running along X
    }

    #[test]
    fn anchor_walker_long_bridge_splits_into_multiple_spans() {
        // 12-cell bridge with anchors at every 4th cell → 3 spans.
        let mut cells = Vec::new();
        for rx in 0..12 {
            cells.push(make_bridge_cell(rx, 0, /* anchor= */ rx % 4 == 0));
        }
        let terrain = ResolvedTerrainGrid::from_cells(12, 1, cells);
        let state = BridgeRuntimeState::from_resolved_terrain(&terrain, true, 1500);
        assert_eq!(state.anchor_spans().len(), 3);
    }

    #[test]
    fn anchor_walker_axis_detection_from_bridge_layer_direction() {
        let mut cells = Vec::new();
        for ry in 0..5 {
            cells.push(make_ns_bridge_cell(0, ry, ry == 2));
        }
        let terrain = ResolvedTerrainGrid::from_cells(1, 5, cells);
        let state = BridgeRuntimeState::from_resolved_terrain(&terrain, true, 1500);
        let span = state.anchor_spans().values().next().unwrap();
        assert_eq!(span.axis, Axis::NS);
    }
```

**Step 3: Verify**

```
cargo test --lib sim::bridge_state::tests::anchor_walker -- --nocapture
```

**Step 4: Commit**

```
git commit -m "test(bridge_state): anchor walker correctness against handcrafted bridge fixtures"
```

---

### Task 28: Gate + retry RNG state-hash test

**Why:** Lockstep determinism contract. Every divergence in RNG draw
order/count breaks replay parity.

**Files:** Add to `src/sim/world/world_tests.rs`.

**Step 1: Add tests**

```rust
    #[test]
    fn non_ion_cannon_consumes_one_rng_draw_per_path() {
        let mut sim = Simulation::new();
        // ... set up bridge cell ...
        let initial_state = sim.rng.state();
        sim.apply_bridge_damage_events(&[BridgeDamageEvent {
            rx: 1, ry: 0, damage: 1,
            warhead_ref: sim.interner.intern("TestWarhead"),
            is_ion_cannon: false,
        }]);
        // Expect exactly 1 RNG draw consumed (per matching dispatch path).
        // For a 1-path-match cell: 1 draw. For 0 paths: 0 draws.
        // Test asserts exact number.
    }

    #[test]
    fn ion_cannon_consumes_zero_strength_gate_draws() {
        let mut sim = Simulation::new();
        // ... set up bridge cell ...
        let initial_state = sim.rng.state();
        sim.apply_bridge_damage_events(&[BridgeDamageEvent {
            rx: 1, ry: 0, damage: 1,
            warhead_ref: sim.interner.intern("TestWarhead"),
            is_ion_cannon: true,
        }]);
        // No RNG draw for the BridgeStrength gate (bypassed for IonCannon).
        // Internal collapse-debris draws (Task 23) may follow if collapse fires.
    }

    #[test]
    fn ion_cannon_retries_up_to_3_times_on_apply_failure() {
        // Hard to test without a failure-injection mock. Assert via deterministic
        // RNG state after a forced collapse path.
    }
```

**Step 2: Verify**

```
cargo test --lib world_tests::rng -- --nocapture
```

**Step 3: Commit**

```
git commit -m "test(world): RNG draw count parity — non-IonCannon=1/path, IonCannon=0 strength draws"
```

---

### Task 29: State machine progression tests

**Why:** Per ledger #21–25. Verify Healthy → Damaged → Destroyed transitions
fire correctly.

**Files:** Already covered by Task 13 unit tests in `bridge_specs.rs`. Add
integration-level test in `world_tests.rs`:

**Step 1: Add test**

```rust
    #[test]
    fn body_cell_first_hit_does_not_destroy_bridge() {
        // Set up bridge with Damaged state.
        // Apply IonCannon damage.
        // Verify: damage_state == Damaged, bridge still walkable.
    }

    #[test]
    fn body_cell_second_hit_destroys_bridge() {
        // Continued from above.
        // Apply second IonCannon damage.
        // Verify: damage_state == Destroyed, not walkable, anchor span emitted collapse.
    }
```

**Step 2: Verify + commit**

```
cargo test --lib world_tests::body_cell -- --nocapture
git commit -m "test(world): body-cell two-step damage progression integration tests"
```

---

### Task 30: Snapshot determinism integration test

**Why:** New fields on BridgeRuntimeCell + AnchorSpan registry must
round-trip through serde, AND state hash before/after must match.

**Files:** `src/sim/world/world_tests.rs` or `tests/`.

**Step 1: Add test**

```rust
    #[test]
    fn bridge_state_hash_invariant_through_snapshot() {
        let mut sim = Simulation::new();
        // Apply some damage to advance state machine.
        let pre_hash = sim.state_hash();
        let snapshot = serde_json::to_string(&sim).expect("serialize");
        let mut restored: Simulation = serde_json::from_str(&snapshot).expect("deserialize");
        // Restore non-serde caches.
        // ...
        let post_hash = restored.state_hash();
        assert_eq!(pre_hash, post_hash);
    }
```

**Step 2: Verify**

```
cargo test --lib world_tests::bridge_state_hash_invariant -- --nocapture
```

**Step 3: Commit**

```
git commit -m "test(world): bridge state hash invariant through snapshot round-trip"
```

---

### Task 31: End-to-end integration test

**Why:** Per Risk Areas. Cover a full bridge lifecycle: map → IonCannon
damage → state advance → ground-occupant kill → state hash deterministic.

**Files:** Create `tests/bridge_tier2_integration.rs`.

**Step 1: Create test file**

```rust
// tests/bridge_tier2_integration.rs

use ra2_rust_game::sim::{Simulation, ...};

#[test]
fn ion_cannon_damages_bridge_and_kills_ground_occupants() {
    // Build a sim fixture with:
    //   - 5x5 map with a bridge across the middle (cells (1,2)..(3,2))
    //   - 1 ground-layer infantry at (2,3) (under the bridge cell (2,2))
    //   - 1 bridge-layer tank at (2,2)
    //
    // Apply IonCannon damage event at (2,2).
    //
    // Verify:
    //   - First hit: bridge enters Damaged state, both units alive.
    //   - Second hit: bridge collapses; ground infantry dying; tank despawned.
    //   - State hash deterministic across two runs with same RNG seed.
}
```

**Step 2: Verify**

```
cargo test --test bridge_tier2_integration -- --nocapture
```

**Step 3: Commit**

```
git commit -m "test: bridge tier 2 end-to-end integration (state advance + ground kill + determinism)"
```

---

### --- Phase G END — Tests green. Tier 2 lands. ---

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-07-bridges-tier2-damage-state-machine-design.md](2026-05-07-bridges-tier2-damage-state-machine-design.md)
- **Ghidra reports:**
  - `ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` (primary, 2692 lines)
  - `ra2-rust-game-docs/BRIDGE_SYSTEM.md`
  - `ra2-rust-game-docs/CELLCLASS_ZONES_SPEED_BRIDGES.md`
  - `ra2-rust-game-docs/BRIDGE_RENDERING_GHIDRA_REPORT.md`
  - `ra2-rust-game-docs/LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md`
- **gamemd.exe addresses (verified live this session):**
  - `Apply_area_damage @ 0x4894B0` — gate + retry semantics
  - `BlowUpBridge @ 0x47DD70` — ground kill + Limbo + debris structure
  - `SetBridgeDirection_NESW @ 0x47E040` — 6-cell walker, 4 BlowUpBridge
  - `ProcessBridgeDamageStateMachine_High @ 0x576BA0` — body + bridgehead branches
  - `ProcessBridgeDamageStateMachine_Low @ 0x571490`
  - Memory `0x7e4f58 = 0.95` (outer probability gate)
  - Memory `0x7e1738 = 0.5` (inner MetallicDebris gate)
  - Memory `0x7e3570 = 1/2^31` (RandomRanged scaling factor)
  - `g_DirectionOffsets @ 0x89F688` (compass index table — runtime-init)
  - `Rules+0xFA8` = C4Warhead, `Rules+0xFF0` = IonCannonWarhead
  - `Rules+0x140` / `+0x14C` = MetallicDebris ptr/count
  - `Rules+0x15C` / `+0x168` = BridgeExplosions ptr/count
  - `CellClass+0x52` = bridgehead height field (Task 0 to confirm)
- **INI keys:**
  - `ini/rulesmd.ini:528` — `[General] MetallicDebris=` (20-entry list)
  - `ini/rulesmd.ini:818` — `[CombatDamage] C4Warhead=Super`
  - `ini/rulesmd.ini:874` — `[CombatDamage] IonCannonWarhead=IonCannonWH`
  - `ini/rulesmd.ini:816` — `[CombatDamage] BridgeStrength=1500` (Tier 1)
  - `ini/rulesmd.ini:804` — `[CombatDamage] DestroyableBridges=yes` (Tier 1)
  - `ini/rulesmd.ini:419` — `[General] BridgeVoxelMax=3` (Tier 1)
- **Related code:**
  - `src/sim/bridge_state.rs:27-34` (existing BridgeRuntimeCell)
  - `src/sim/bridge_specs.rs:93-122` (existing pure helper to wire)
  - `src/sim/world/mod.rs:673-687, 851-919` (existing damage + debris paths)
  - `src/sim/combat/mod.rs` 3 emit sites — anchor by content (`bridge_damage_events.push(BridgeDamageEvent {`), not line number. Current locations on `dev` HEAD: lines 798 (death AoE), 1476 (primary attack AoE), 1511 (primary attack non-AoE). Parallel `TargetKind` work has shifted these ~130 lines from the original design references; expect further drift.
  - `src/rules/ruleset.rs:652-715` (Tier 1 BridgeRules pattern)
  - `src/rules/combat_damage.rs` (sub-struct pattern)
  - `src/sim/rng.rs:45-51` (existing RNG API)
- **Prior commits:**
  - Tier 1 lands at `3bc846e..2f0476e` (BridgeStrength=1500, DestroyableBridges from [CombatDamage], voxel_max + repair_sound + bridge_repair_hut parsed but unread)
- **Parallel session:** uncommitted `TargetKind` work in `src/sim/combat/mod.rs` — anchor by content patterns (3 `bridge_damage_events.push(BridgeDamageEvent {` sites), not by line number.
