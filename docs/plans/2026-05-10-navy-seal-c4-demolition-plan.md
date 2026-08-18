# Navy SEAL / Tanya C4 Building Demolition — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Player right-clicks an enemy structure with SEAL/Tanya selected; the unit walks
to the building's cell, claims the plant, the building's timer fires C4Warhead damage
after C4Delay (default 27 ticks), the building dies, and the SEAL survives and walks one
cell away. Match `gamemd.exe` observable behavior.

**Architecture:** Two state lanes — `c4_plant: Option<C4PlantState>` on the attacker
(walk-up intent; mirrors engineer-capture's `capture_target`), and
`pending_c4_detonation: Option<PendingC4Detonation>` on the building (the timer; mirrors
gamemd's building-side `+0x528/+0x530/+0x540/+0x6df` state). New `tick_c4_plants`
slotted immediately after `tick_capture_orders` in `advance_tick` Phase 5.

**Design Doc:** [2026-05-10-navy-seal-c4-demolition-design.md](2026-05-10-navy-seal-c4-demolition-design.md)

---

## Grounding Summary

- **Docs (R1):** `ra2-rust-game-docs/NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md` — full pipeline,
  flag map, struct offsets, INI defaults, active-in-YR classification. Confidence HIGH on
  dispatch graph, cursor path, INI offsets; MEDIUM on plant-timer frame mechanics
  (resolved live during brainstorm).
- **Ghidra (R2):** `BuildingClass::Update @ 0x0043fb20` and `Mission_Enter @ 0x005196a0`
  decompiled live during brainstorm to resolve Open Question 2:
  - Marker `+0x6df` is **never cleared** in the C4 path (only in the BridgeRepairHut
    sibling branch). Detonation fires through attacker death and through Iron Curtain.
  - Building-side detonation damage = `building.Health` (current HP) × Verses, which is
    why Super warhead one-shots any building regardless of armor.
  - The 3 `Apply_area_damage` calls in Mission_Enter only run AFTER the building has
    died from its own update tick; they handle overlay chain reaction and cell-AoE
    secondary effects.
  - Ghidra MCP disconnected after this verification; no further binary work needed.
- **Repo pattern (R3):** Engineer capture is the exact analog —
  [Command::CaptureBuilding](../../src/sim/command.rs#L120-L126),
  [GameEntity.capture_target](../../src/sim/game_entity.rs#L199-L201),
  [world_commands.rs:861-915](../../src/sim/world/world_commands.rs#L861-L915) (dispatch),
  [world_orders.rs:151-209](../../src/sim/world/world_orders.rs#L151-L209) (tick handler).
- **INI keys (R4):** Three new keys to parse:
  - `[InfantryName] C4=` — default `no`. Set on `[GHOST]`, `[TANY]`, `[PTROOP]`.
  - `[BuildingName] CanC4=` — default `yes`. Set to `no` on `[CAMISC01]`, `[CAMISC02]`,
    `[CAMSC09]`, `[CAMSC10]`.
  - `[BuildingName] InvisibleInGame=` — default `no`. No stock building sets it.
  - `[CombatDamage] C4Delay=` — default `0.03` minutes = 27 ticks @ 15 fps.
  - `[CombatDamage] C4Warhead=` — already parsed at
    [bridge_warheads.rs](../../src/rules/bridge_warheads.rs); reused as-is.
- **Discovered during grounding:**
  - `CursorId::Demolish` **already exists** at [app_types.rs:88](../../src/app_types.rs#L88)
    and the frames are already loaded into the atlas at
    [cursor_atlas.rs:202-208](../../src/render/cursor_atlas.rs#L202-L208). Only
    `CursorFeedbackKind::Demolish` + the `cursor_id_for_feedback` arm need to be added.
  - `apply_aoe_damage` at [combat_aoe.rs:33](../../src/sim/combat/combat_aoe.rs#L33)
    early-returns on `cell_spread <= 0`. Super warhead has `CellSpread=0` by default, so
    we cannot reuse it for C4 detonation as-is. Plan uses **direct-damage to the building
    entity**, which exactly matches what gamemd's `BuildingClass::Update` does
    (`TakeDamage(damage=building.Health, ...)` at the +0x6df handler).
  - `Animation::switch_to(SequenceKind::Attack)` at
    [animation.rs:204](../../src/sim/animation.rs#L204) drives the FireUp sequence from
    `art.ini`. Used for the plant animation.
- **Git state check:** `git log -10` on every design touch-point shows no commits since
  the design doc — design's "current state" claims still hold.
- **Resolved during review (formerly "still unknown"):**
  - `building.position.rx/ry` is the **anchor cell**; multi-cell buildings occupy
    additional cells via `building_footprint_cells()` ([world_spawn.rs:240-247](../../src/sim/world/world_spawn.rs#L240-L247)).
  - Pathfinder treats building footprint cells as blocked
    ([cell_entry.rs:394](../../src/sim/pathfinding/cell_entry.rs#L394)), so the
    SEAL cannot walk INTO the building's cell. **Decision: use Chebyshev-≤-1
    adjacency** (matches engineer-capture pattern at
    [world_orders.rs:189](../../src/sim/world/world_orders.rs#L189)).
    Small visual parity drift documented in the parity ledger.

## Key Technical Decisions

- **Split state across two entities** (attacker has `c4_plant`, building has
  `pending_c4_detonation`): mirrors gamemd's layout and produces correct
  cancellation behavior (attacker death + IC) without explicit handling.
  **Confidence: high** — **Source:** verified Ghidra
  `BuildingClass::Update @ 0x0043fb20` during brainstorm.
- **Direct damage to building entity at detonation** (not via `apply_aoe_damage`):
  matches `BuildingClass::Update`'s TakeDamage call exactly. Super warhead's
  CellSpread=0 would block `apply_aoe_damage` anyway. **Confidence: high** —
  **Source:** verified Ghidra `BuildingClass::Update @ 0x0043fb20`.
- **Damage value = `building.health.current`** at detonation tick: one-shot kill
  regardless of Verses. **Confidence: high** — **Source:** verified Ghidra
  (`iStack_28 = this->Health; vtable[+0x16c](&iStack_28, ...)`).
- **Marker permanence** (don't clear `pending_c4_detonation` on attacker death or
  IC): **Confidence: high** — **Source:** verified Ghidra OQ2.
- **Clear `c4_plant` on Command::Move and Command::Stop**: mirrors gamemd's
  SEAL.Mission transition from 0x11 → 2. **Confidence: high** — **Source:**
  inferred from `Mission_Enter` early-return when SEAL.Mission != 0x11.
- **C4Delay parsed as f64 minutes, converted to u32 ticks at 15 fps**:
  `0.03 × 60 × 15 = 27` ticks. **Confidence: high** — **Source:** research §5.
- **`can_c4` default = `true` only for Building category**: matches
  `BuildingTypeClass::ReadINI_Water`'s default of `1`. **Confidence: high** —
  **Source:** research §10.B.

## Open Questions

### Resolved During Planning

- **OQ: Cleanup of `+0x6df` on attacker death?** → Verified: never cleared in the C4
  path. Detonation fires regardless. (Resolved via live Ghidra in brainstorm.)
- **OQ: `apply_aoe_damage` for C4?** → No. Super warhead's CellSpread=0 blocks the
  AoE helper; direct-damage the building entity (matches gamemd's BuildingClass::Update).
- **OQ: Animation hookup?** → `Animation::switch_to(SequenceKind::Attack)` drives the
  FireUp sequence. Standard call.
- **OQ: CursorId::Demolish exists?** → Yes, already loaded at cursor_atlas.rs:202-208.

### Deferred to Implementation

- **Selling-in-progress building** (Mission==0x13 exclusion in gamemd's Mission_Enter):
  not modeled; rare edge case. Leave a `// TODO(parity): reject C4 plant on
  selling-in-progress buildings` comment in command dispatch.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/rules/object_type.rs` | Add 3 fields (`c4`, `can_c4`, `invisible_in_game`) + parsers + default rule for `can_c4` |
| Modify | `src/rules/ruleset.rs` | Add `c4_delay_ticks: u32` + parser from `[CombatDamage] C4Delay=` |
| Modify | `src/sim/components.rs` | Add `C4PlantState` and `PendingC4Detonation` structs |
| Modify | `src/sim/game_entity.rs` | Add `c4_plant` and `pending_c4_detonation` fields + defaults |
| Modify | `src/sim/world/world_hash.rs` | Hash new fields for lockstep |
| Modify | `src/sim/command.rs` | Add `Command::PlantC4` variant |
| Modify | `src/sim/world/world_commands.rs` | Dispatch `Command::PlantC4`; clear `c4_plant` on Move/Stop |
| Modify | `src/sim/world/world_orders.rs` | New `tick_c4_plants` function (Phase 1 walk-up, Phase 2 detonation, scatter helper) |
| Modify | `src/sim/world/mod.rs` | Call `tick_c4_plants` in `advance_tick`; add `SimSoundEvent::C4Planted` |
| Modify | `<app-side audio dispatcher>` | Map `SimSoundEvent::C4Planted` → `[SealPlaceBomb]` (Task 8a) |
| Modify | `src/app_types.rs` | Add `CursorFeedbackKind::Demolish` |
| Modify | `src/app_cursor.rs` | Replace `sabotage_cursor` cursor branch with `c4 && can_c4 && !invisible_in_game`; map Feedback::Demolish → CursorId::Demolish |
| Modify | `src/app_context_order.rs` | New right-click branch emitting `Command::PlantC4` |
| Create | `src/sim/world/world_orders_c4_tests.rs` (or extend `world_tests.rs`) | Unit + integration tests |

## Interface Changes

- **`Command` enum gains `PlantC4 { attacker_id: u64, target_building_id: u64 }`**.
  Consumers: serde (replay/save), `world_commands.rs` dispatch, `command.rs` hashing
  (via PartialEq+Eq derive). Replay files predating this commit have no `PlantC4`
  entries — no migration concern.
- **`GameEntity` gains two `Option<...>` fields**. Consumers: serde
  (`#[serde(default)]` ensures backwards compat for save files), `world_hash`, the
  new tick handler. No call sites outside the new handler need to touch the fields
  directly.
- **`ObjectType` gains 3 fields** — same serde default story.
- **`CursorFeedbackKind::Demolish`** — new variant. Consumers: `cursor_id_for_feedback`
  match (line 442) gains a new arm.

## Sim Checklist

- [ ] All math integer / `fixed`-point — no f32/f64 in game logic (only INI parse uses
      f64 for C4Delay→ticks conversion, which happens once at load and stores a u32).
- [ ] New state included in deterministic state hash (`c4_plant`,
      `pending_c4_detonation`).
- [ ] No sim/ dependencies on render/ui/sidebar/audio/net (the audio cue is queued
      via `sim.sound_events` like other sim-emitted sounds).
- [ ] Tick ordering: `tick_c4_plants` slots immediately after `tick_capture_orders`,
      before `tick_attack_pursuit` and combat. Documented in `advance_tick` comment.
- [ ] BTreeMap iteration order: tick handler uses `keys_sorted()` for both walk-up
      and detonation phases.

## Risk Areas

- **Determinism**: new tick handler must be in deterministic phase order and iterate
  in sorted order. State hash must include both new fields.
- **Cursor regression**: replacing the `sabotage_cursor`-driven Enter cursor with
  `c4`-driven Demolish for SEAL/Tanya. Tanya/SEAL are the only stock units with
  `SabotageCursor=yes`. Modders using SabotageCursor on a non-C4 unit will lose the
  Enter cursor — document in code comment as intentional (cursor is now C4-gated,
  matching gamemd action 0x10).
- **Engineer-capture interaction**: a unit with both `c4=true` and `engineer=true`
  would currently be ambiguous. None exist in stock YR. Plan orders the C4 click
  branch BEFORE the engineer branch in `app_context_order.rs` so C4 wins.
- **`apply_aoe_damage` CellSpread=0 quirk**: documented as known limitation; the C4
  path doesn't use it. Direct-damage path is what gamemd uses anyway.
- **Save/replay compat**: `#[serde(default)]` on new fields preserves loading of
  pre-feature saves.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 2 | C4Delay default = 27 ticks (0.03 minutes × 15 fps) | Player times raids around this delay | Unit test: `c4_delay_ticks == 27` with retail rulesmd.ini |
| Task 8 | Plant timer starts when SEAL is Chebyshev-≤-1 adjacent to the building | Pathfinder treats building footprints as blocked; matches engineer-capture's pattern. **Parity drift accepted**: SEAL stands next to the building during plant instead of inside it (gamemd has SEAL walk into the cell). Audio + animation + detonation + survival are unchanged. | Integration test #1 |
| Task 8 | Damage = `building.health.current` (one-shot kill) | C4 always destroys at full HP — player relies on this | Integration test #1: assert dying=true at detonation tick |
| Task 8 | Marker never cleared on attacker death | SEAL killed during plant → building still dies | Integration test #2 |
| Task 8 | Damage fires every tick once timer elapsed (IC behavior) | IC-protected building dies once IC drops | Integration test #3 |
| Task 8 | Second SEAL on already-planted target idles | Player observation: second SEAL hovers, no double-plant | Integration test #4 |
| Task 8 | Post-detonation scatter direction = `(((tick >> 12) + 1) >> 1) & 7` | SEAL walks one cell post-plant in deterministic direction | Integration test (add to Task 13): assert SEAL on adjacent cell after detonation, hash deterministic across runs |
| Task 8 | SEAL survives plant | Critical player observable | Integration test #1: SEAL still in entity store at end |
| Task 8 | Plant animation = Attack/FireUp sequence | Player sees SEAL crouch during plant | Visual check during manual playtest |
| Task 8a | SealPlaceBomb spatial sound on plant claim | Audible feedback at plant start | Manual playtest #5; sim test asserts SimSoundEvent::C4Planted queued |
| Task 11 | Cursor = Demolish (not Enter) on valid C4 targets | Distinct mouse.shp frames — player notices | Manual playtest: hover SEAL on Allied Barracks |
| Task 11 | Cursor = Attack (not Demolish) on `CanC4=no` building | CAMISC01 (Oil Derrick) etc. | Manual playtest |
| Task 11 | Cursor = Attack (not Demolish) on iron-curtained target | IC blocks at issue time | Manual playtest |
| Task 12 | EVA voice `SealSpecialAttack` on plant order | Audible feedback at command issue | Manual playtest |
| Task 8 | Sound `SealPlaceBomb` on plant claim | Audible feedback at plant start | Manual playtest |

---

## Tasks

### Task 1: Add `c4`, `can_c4`, `invisible_in_game` fields to ObjectType

**Why:** All downstream gating reads these flags. Parse them first so the rest of the
system can rely on them.

**Files:**
- Modify: `src/rules/object_type.rs`

**Pattern:** Mirror existing capability-flag fields (engineer, capturable, occupier).
The `can_c4` default-true-for-buildings pattern mirrors `toggle_power` /
`powered` (lines 929-935).

**Step 1: Add the three fields to `ObjectType` struct.**

Locate the cursor-capability section around line 545 (just after `sabotage_cursor`).
Add:

```rust
    /// `C4=yes` on InfantryType. Gates the player-issued C4 plant mission path
    /// (SEAL, Tanya, Psi-Corp Trooper). Distinct from `sabotage_cursor`, which
    /// is now purely a modder-flag for cursor display on weapons; the live
    /// cursor + click behavior is driven by `c4 + can_c4` instead.
    pub c4: bool,

    /// `CanC4=yes` on BuildingType. When false, the building cannot be C4'd by
    /// SEAL/Tanya/PTROOP. Default `true` for buildings, `false` for non-buildings.
    /// Stock buildings opting out: CAMISC01 (Oil Derrick), CAMISC02 (Barrel),
    /// CAMSC09, CAMSC10 (McBurger Kong).
    pub can_c4: bool,

    /// `InvisibleInGame=yes` on BuildingType. Logical-only buildings (e.g., bridge
    /// anchors) that should not receive C4 or other interaction cursors. No stock
    /// targetable building sets this.
    pub invisible_in_game: bool,
```

**Step 2: Add parsers in `ObjectType::from_ini_section` around line 923 (just after
`sabotage_cursor:`).**

```rust
            c4: section.get_bool("C4").unwrap_or(false),
            can_c4: section
                .get_bool("CanC4")
                .unwrap_or(category == ObjectCategory::Building),
            invisible_in_game: section.get_bool("InvisibleInGame").unwrap_or(false),
```

**Step 3: Add unit tests.**

Append to the existing `#[cfg(test)] mod tests` block in `object_type.rs`. Use the
inline `IniFile::from_str(...).section(...).unwrap()` pattern that the existing tests
use (e.g., [object_type.rs:1364-1366](../../src/rules/object_type.rs#L1364-L1366) —
`test_size_defaults_by_category`). The verified signature of `from_ini_section` is
`(name, section, category)` — name first.

```rust
    #[test]
    fn c4_flag_parses_from_ini() {
        let ini = IniFile::from_str("[GHOST]\nC4=yes\n");
        let section = ini.section("GHOST").unwrap();
        let obj = ObjectType::from_ini_section("GHOST", section, ObjectCategory::Infantry);
        assert!(obj.c4);
    }

    #[test]
    fn c4_defaults_to_false() {
        let ini = IniFile::from_str("[E1]\n");
        let section = ini.section("E1").unwrap();
        let obj = ObjectType::from_ini_section("E1", section, ObjectCategory::Infantry);
        assert!(!obj.c4);
    }

    #[test]
    fn can_c4_defaults_to_true_for_buildings() {
        let ini = IniFile::from_str("[GAPILE]\n");
        let section = ini.section("GAPILE").unwrap();
        let obj = ObjectType::from_ini_section("GAPILE", section, ObjectCategory::Building);
        assert!(obj.can_c4);
    }

    #[test]
    fn can_c4_defaults_to_false_for_non_buildings() {
        let ini = IniFile::from_str("[E1]\n");
        let section = ini.section("E1").unwrap();
        let obj = ObjectType::from_ini_section("E1", section, ObjectCategory::Infantry);
        assert!(!obj.can_c4);
    }

    #[test]
    fn can_c4_no_overrides_default() {
        let ini = IniFile::from_str("[CAMISC01]\nCanC4=no\n");
        let section = ini.section("CAMISC01").unwrap();
        let obj = ObjectType::from_ini_section("CAMISC01", section, ObjectCategory::Building);
        assert!(!obj.can_c4);
    }

    #[test]
    fn invisible_in_game_defaults_to_false() {
        let ini = IniFile::from_str("[GAPILE]\n");
        let section = ini.section("GAPILE").unwrap();
        let obj = ObjectType::from_ini_section("GAPILE", section, ObjectCategory::Building);
        assert!(!obj.invisible_in_game);
    }
```

**Step 4: Verify.**

```
cargo test -p ra2_rust --lib rules::object_type::tests -- --nocapture
```

Expected: 6 new tests pass.

**Step 5: Commit.**

```
git add src/rules/object_type.rs
git commit -m "rules/object_type: parse C4, CanC4, InvisibleInGame flags"
```

---

### Task 2: Add `c4_delay_ticks` to RuleSet

**Why:** The detonation tick handler needs this constant. Parse once at load.

**Files:**
- Modify: `src/rules/ruleset.rs`

**Pattern:** Mirror existing `[CombatDamage]`-section integer fields. The double-to-ticks
conversion is one-time at load.

**Step 1: Add the field to `RuleSet` struct.**

Locate the `[CombatDamage]`-related fields near `bridge_warheads` (around line 1163).
Add:

```rust
    /// `[CombatDamage] C4Delay=`. Default `0.03` minutes = 27 ticks @ 15 fps.
    /// Time between SEAL plant claim and detonation. Stored as integer ticks
    /// (not minutes) so the per-tick comparison stays integer/lockstep-safe.
    pub c4_delay_ticks: u32,
```

**Step 2: Add a constant for the conversion** in `ruleset.rs` or a shared
constants file (search for existing tick-rate constants):

```rust
/// Simulation tick rate. Mirrors gamemd.exe's 15 fps logic frame counter.
const SIM_TICKS_PER_SECOND: u32 = 15;
```

If a constant already exists (e.g., `TICKS_PER_SECOND` in `sim/world/mod.rs`),
**reuse it** rather than redeclare.

**Step 3: Parse the value.**

Locate where `bridge_warheads` is parsed (around line 1318). Add right after:

```rust
        // [CombatDamage] C4Delay = minutes (double). Default 0.03 = 27 ticks @ 15 fps.
        let c4_delay_ticks: u32 = ini
            .section("CombatDamage")
            .and_then(|s| s.get("C4Delay"))
            .and_then(|v| v.trim().parse::<f64>().ok())
            .map(|minutes| (minutes * 60.0 * SIM_TICKS_PER_SECOND as f64).round() as u32)
            .unwrap_or(27); // 0.03 × 60 × 15 = 27
```

**Step 4: Wire into the struct construction** around line 1386:

```rust
            bridge_warheads,
            c4_delay_ticks,
            // ... existing fields
```

**Step 5: Add unit tests.**

Append to `rules/ruleset.rs` tests. Verified signature: `RuleSet::from_ini(ini: &IniFile)
-> Result<Self, RulesError>` — single arg, returns `Result`. Mirror the pattern at
[ruleset.rs:2026](../../src/rules/ruleset.rs#L2026):
`let rules = RuleSet::from_ini(&ini).expect("Should parse");`.

```rust
    #[test]
    fn c4_delay_defaults_to_27_ticks() {
        let ini = IniFile::from_str("");
        let rules = RuleSet::from_ini(&ini).expect("parse");
        assert_eq!(rules.c4_delay_ticks, 27);
    }

    #[test]
    fn c4_delay_parses_double_minutes_to_ticks() {
        let ini = IniFile::from_str("[CombatDamage]\nC4Delay=0.1\n");
        let rules = RuleSet::from_ini(&ini).expect("parse");
        // 0.1 minutes × 60 × 15 = 90 ticks
        assert_eq!(rules.c4_delay_ticks, 90);
    }

    #[test]
    fn c4_delay_retail_default_value() {
        let ini = IniFile::from_str("[CombatDamage]\nC4Delay=0.03\n");
        let rules = RuleSet::from_ini(&ini).expect("parse");
        // 0.03 × 60 × 15 = 27 (.round())
        assert_eq!(rules.c4_delay_ticks, 27);
    }
```

**Step 6: Verify.**

```
cargo test -p ra2_rust --lib rules::ruleset::tests::c4_delay -- --nocapture
```

Expected: 3 new tests pass.

**Step 7: Commit.**

```
git add src/rules/ruleset.rs
git commit -m "rules/ruleset: parse [CombatDamage] C4Delay= (minutes → ticks)"
```

---

### Task 3: Add `C4PlantState` and `PendingC4Detonation` components

**Why:** Both new lanes of state need types defined before any handler can use them.
This task only adds the types; fields are added to `GameEntity` in Task 4.

**Files:**
- Modify: `src/sim/components.rs`

**Pattern:** Mirror existing per-entity state structs in `components.rs` —
`DriveTrackState`, `DockState`, etc. — small, `Copy`, derive serde + Eq + Hash for
state-hash inclusion.

**Step 1: Add both structs to `src/sim/components.rs`.**

Append near the bottom of the file, before the `#[cfg(test)]` block:

```rust
/// Per-attacker walk-up intent for the C4 plant mission.
///
/// Mirrors gamemd's SEAL/Tanya/PTROOP behavior: while this is `Some`, the
/// unit pathfinds toward the target building. On arrival at the target's
/// cell, `tick_c4_plants` claims the plant by setting
/// `PendingC4Detonation` on the building. This state is cleared when the
/// player retasks the unit (Move/Stop) or when the target is lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct C4PlantState {
    pub target_building_id: u64,
}

/// Per-building C4 detonation timer.
///
/// Set by `tick_c4_plants` when an attacker with `c4_plant` arrives on the
/// building's cell. Once set, the building's update tick fires C4Warhead
/// damage every tick after `plant_start_tick + rules.c4_delay_ticks`, using
/// `damage = current_hp` for guaranteed one-shot kill.
///
/// **Never cleared** in the C4 path — matches gamemd's `+0x6df` marker
/// semantics. When the building dies, this state is despawned with it.
/// IronCurtain on the building does NOT clear this; damage attempts get
/// nullified by `is_invulnerable` each tick until IC expires, at which
/// point the next damage tick kills the building.
///
/// Verified live during brainstorm: `BuildingClass::Update @ 0x0043fb20`
/// reads `field_0x528` (plant start frame), `field_0x530` (delay frames),
/// `field_0x540` (attacker ptr), `field_0x6df` (marker). The C4 path never
/// writes `field_0x6df = 0` — only the BridgeRepairHut sibling path does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PendingC4Detonation {
    pub plant_start_tick: u64,
    /// Original attacker for kill-credit. May refer to a despawned entity
    /// at detonation time; in that case the credit is unattributed (the
    /// binary uses a dangling pointer; we resolve gracefully to None).
    pub attacker_id: u64,
}
```

**Step 2: Add a quick `assert_send_sync` test alongside the existing one** (which I
saw at line 651 for `OrderIntent`):

```rust
    #[test]
    fn c4_state_types_are_send_sync_copy() {
        assert_send_sync::<C4PlantState>();
        assert_send_sync::<PendingC4Detonation>();
        // Compile-time Copy assertion via fn-bound:
        fn _assert_copy<T: Copy>() {}
        _assert_copy::<C4PlantState>();
        _assert_copy::<PendingC4Detonation>();
    }
```

**Step 3: Verify.**

```
cargo build -p ra2_rust
cargo test -p ra2_rust --lib sim::components::tests::c4_state -- --nocapture
```

Expected: build succeeds, test passes.

**Step 4: Commit.**

```
git add src/sim/components.rs
git commit -m "sim/components: add C4PlantState and PendingC4Detonation types"
```

---

### Task 4: Add `c4_plant` and `pending_c4_detonation` fields to `GameEntity`

**Why:** These are the two state lanes — one on the attacker, one on the building.
Both default to `None`.

**Files:**
- Modify: `src/sim/game_entity.rs`

**Pattern:** Mirror `capture_target: Option<u64>` at line 199-201. Use
`#[serde(default)]` for save-file backwards compat.

**Step 1: Add imports.**

Locate the existing `use` block at the top of `game_entity.rs`. Look for the
import that brings in `OrderIntent`, `Position`, etc. (around line 24). Extend the
import to include the new types:

```rust
use crate::sim::components::{
    // ... existing imports ...
    C4PlantState, PendingC4Detonation,
    HarvestOverlay, Health, MovementTarget, OrderIntent, Position, VoxelAnimation,
};
```

**Step 2: Add the two fields to `GameEntity`.**

Locate `capture_target: Option<u64>` at line 199-201. Immediately after it, add:

```rust
    /// Active C4 plant intent on this attacker. Set by `Command::PlantC4`,
    /// cleared on arrival (after the building's pending detonation is set),
    /// when the player retasks the unit, or when the target is lost.
    /// `None` for non-C4 attackers or attackers not currently planting.
    #[serde(default)]
    pub c4_plant: Option<C4PlantState>,

    /// Active C4 detonation timer on this building. Set by `tick_c4_plants`
    /// when a C4-capable attacker arrives on this building's cell. Once set,
    /// `tick_c4_plants` Phase 2 fires C4Warhead damage every tick after
    /// `plant_start_tick + rules.c4_delay_ticks` until the building dies.
    /// Never cleared in the C4 path — matches gamemd marker semantics.
    /// `None` for non-buildings or buildings not currently being C4'd.
    #[serde(default)]
    pub pending_c4_detonation: Option<PendingC4Detonation>,
```

**Step 3: Add defaults in `GameEntity::new`.**

Locate the `GameEntity::new` constructor (search for `fn new(`, around line 217).
Find the existing `capture_target: None,` and add right after:

```rust
            c4_plant: None,
            pending_c4_detonation: None,
```

**Step 4: Verify.**

```
cargo build -p ra2_rust
```

Expected: clean build. No new tests yet (covered by Task 5 hash test and integration
tests later).

**Step 5: Commit.**

```
git add src/sim/game_entity.rs
git commit -m "sim/game_entity: add c4_plant + pending_c4_detonation fields"
```

---

### Task 5: Hash new fields in `world_hash.rs`

**Why:** Lockstep determinism requires every replicated entity field to be in the state
hash. Skipping this would silently produce desyncs.

**Files:**
- Modify: `src/sim/world/world_hash.rs`

**Pattern:** Mirror the existing `entity.capture_target.hash(hasher);` line at 382.

**Step 1: Locate the per-entity hash block.**

Find line 382 (`entity.capture_target.hash(hasher);`). Add immediately after:

```rust
        entity.c4_plant.hash(hasher);
        entity.pending_c4_detonation.hash(hasher);
```

Since both types `derive(Hash)`, `Option<T>: Hash` works automatically.

**Step 2: Add a determinism test.**

Append to the `#[cfg(test)] mod tests` block in `world_hash.rs` (or, if tests live
elsewhere, in `world_tests.rs`):

```rust
    #[test]
    fn c4_state_changes_hash() {
        use crate::sim::components::{C4PlantState, PendingC4Detonation};
        let mut sim = build_minimal_sim();
        // Spawn one entity, capture initial hash.
        let id = spawn_test_infantry(&mut sim, "GHOST", 10, 10);
        let h_initial = sim.state_hash();

        // Mutate c4_plant — hash must change.
        sim.entities.get_mut(id).unwrap().c4_plant = Some(C4PlantState {
            target_building_id: 99,
        });
        let h_with_plant = sim.state_hash();
        assert_ne!(h_initial, h_with_plant, "c4_plant must affect state hash");

        // Mutate pending_c4_detonation — hash must change again.
        sim.entities.get_mut(id).unwrap().pending_c4_detonation =
            Some(PendingC4Detonation {
                plant_start_tick: 100,
                attacker_id: 7,
            });
        let h_with_pending = sim.state_hash();
        assert_ne!(h_with_plant, h_with_pending,
                   "pending_c4_detonation must affect state hash");
    }
```

If `build_minimal_sim` / `spawn_test_infantry` helpers exist (search nearby tests
for the conventions), use them. Otherwise, model after the closest existing test in
`world_hash.rs` or `world_tests.rs`.

**Step 3: Verify.**

```
cargo test -p ra2_rust --lib c4_state_changes_hash -- --nocapture
```

Expected: test passes.

**Step 4: Commit.**

```
git add src/sim/world/world_hash.rs
git commit -m "sim/world_hash: include c4_plant + pending_c4_detonation"
```

---

### Task 6: Add `Command::PlantC4` variant and dispatch

**Why:** The player's right-click translates to this command via
`app_context_order.rs`; the sim consumes it here. Must validate ownership,
attacker `c4` flag, target eligibility, IC, and fog visibility.

**Files:**
- Modify: `src/sim/command.rs`
- Modify: `src/sim/world/world_commands.rs`

**Pattern:** Mirror `Command::CaptureBuilding` (command.rs:120-126) and its dispatch
(world_commands.rs:861-915).

**Step 1: Add the variant to `Command`.**

In `src/sim/command.rs`, around line 124 (right after `CaptureBuilding`), add:

```rust
    /// Order a C4-capable infantry (SEAL / Tanya / Psi-Corp Trooper) to plant
    /// on an enemy building. The unit walks to the building's cell; on arrival
    /// the building's `pending_c4_detonation` is set; after `C4Delay` ticks the
    /// building takes full-HP damage with C4Warhead and dies. The attacker
    /// survives and scatters one cell. Gating happens in `world_commands` —
    /// attacker must have `C4=yes`, target must be a `CanC4=yes` building, not
    /// invisible-in-game, not iron-curtained, not in fog.
    PlantC4 {
        attacker_id: u64,
        target_building_id: u64,
    },
```

**Step 2: Add dispatch in `world_commands.rs`.**

Insert before `Command::CaptureBuilding` (line 861). Copy the engineer block's
structure verbatim and adapt:

```rust
            Command::PlantC4 {
                attacker_id,
                target_building_id,
            } => {
                let Some(rules) = rules else { return false };
                if !self.entity_owned_by_id(command_owner, *attacker_id) {
                    return false;
                }
                if self
                    .entities
                    .get(*attacker_id)
                    .is_some_and(|e| e.is_deployed())
                {
                    return false;
                }
                // Validate attacker has C4=yes flag.
                let c4_ok = self.entities.get(*attacker_id).and_then(|e| {
                    let obj = rules.object(self.interner.resolve(e.type_ref))?;
                    obj.c4.then_some(())
                });
                if c4_ok.is_none() {
                    return false;
                }
                // Validate target is a CanC4, non-invisible enemy building, not iron-curtained.
                // TODO(parity): also reject selling-in-progress buildings (Mission==0x13);
                // requires building Mission state which isn't modeled yet.
                let target_info = self.entities.get(*target_building_id).and_then(|b| {
                    if b.category != crate::map::entities::EntityCategory::Structure {
                        return None;
                    }
                    if b.dying {
                        return None;
                    }
                    let obj = rules.object(self.interner.resolve(b.type_ref))?;
                    if !obj.can_c4 || obj.invisible_in_game {
                        return None;
                    }
                    if crate::sim::superweapon::invulnerability::is_invulnerable(
                        b.invulnerability.as_ref(),
                        self.tick as u32,
                    ) {
                        return None;
                    }
                    Some((b.position.rx, b.position.ry, b.owner))
                });
                let Some((trx, try_, target_owner)) = target_info else {
                    return false;
                };
                // Enemy-only.
                if crate::map::houses::are_houses_friendly(
                    &self.house_alliances,
                    command_owner,
                    self.interner.resolve(target_owner),
                ) {
                    return false;
                }
                // Clear conflicting state and set c4_plant.
                if let Some(e) = self.entities.get_mut(*attacker_id) {
                    e.attack_target = None;
                    e.order_intent = None;
                    e.dock_state = None;
                    e.capture_target = None;
                    e.c4_plant = Some(crate::sim::components::C4PlantState {
                        target_building_id: *target_building_id,
                    });
                }
                // Issue movement toward the building's cell. Copy the same
                // path-issue scaffold the CaptureBuilding branch uses below.
                let info = self.resolve_move_info(*attacker_id, Some(rules));
                let speed = info
                    .as_ref()
                    .map(|i| i.speed)
                    .unwrap_or(ra2_speed_to_leptons_per_second(4));
                let speed_type = info
                    .as_ref()
                    .map(|i| i.speed_type)
                    .unwrap_or(crate::rules::locomotor_type::SpeedType::Foot);
                let crusher = info.as_ref().map_or(false, |i| i.mover_is_crusher);
                let (entity_blocks, entity_block_map) =
                    crate::sim::movement::bump_crush::build_entity_block_set(
                        &self.entities,
                        command_owner,
                        &self.house_alliances,
                        &self.interner,
                        Some(rules),
                    );
                if let Some(grid) = path_grid {
                    let cost_grid = self.terrain_costs.get(&speed_type);
                    movement::issue_move_command_with_layered(
                        &mut self.entities,
                        grid,
                        *attacker_id,
                        (trx, try_),
                        speed,
                        false,
                        Some(&entity_blocks),
                        Some(&entity_block_map),
                        self.resolved_terrain.as_ref(),
                        cost_grid,
                        crusher,
                    );
                }
                true
            }
```

(Match the exact argument list of `issue_move_command_with_layered` — copy the
engineer's call site at world_commands.rs:937-948 verbatim to avoid signature drift.)

**Step 3: Verify.**

```
cargo build -p ra2_rust
cargo test -p ra2_rust --lib sim::command -- --nocapture
```

Expected: clean build. No new functional tests at this step; covered by integration
tests in Task 13.

**Step 4: Commit.**

```
git add src/sim/command.rs src/sim/world/world_commands.rs
git commit -m "sim/command: add Command::PlantC4 + dispatch validation"
```

---

### Task 7: Clear `c4_plant` on Move and Stop

**Why:** When the player retasks the SEAL during walk-up, the plant intent is
abandoned (matches gamemd's SEAL.Mission transition from 0x11 → 2). Note:
`pending_c4_detonation` on the BUILDING is NOT cleared — once the plant is
claimed, the timer runs regardless of attacker re-tasking.

**Files:**
- Modify: `src/sim/world/world_commands.rs`

**Pattern:** Same lines that already clear `attack_target` and `order_intent`.

**Step 1: Patch `Command::Move`.**

At lines 136-137 in `world_commands.rs`:

```rust
                if let Some(e) = self.entities.get_mut(*entity_id) {
                    e.attack_target = None;
                    e.order_intent = None;
                    e.c4_plant = None;   // NEW: cancel plant walk-up on new Move
                    // ... rest unchanged
                }
```

**Step 2: Patch `Command::Stop`.**

At lines 275-277:

```rust
                if let Some(e) = self.entities.get_mut(*entity_id) {
                    e.movement_target = None;
                    e.attack_target = None;
                    e.order_intent = None;
                    e.dock_state = None;
                    e.c4_plant = None;   // NEW: cancel plant walk-up on Stop
                }
```

**Step 3: Verify.**

```
cargo build -p ra2_rust
```

Expected: clean build. Behavioral verification in Task 13.

**Step 4: Commit.**

```
git add src/sim/world/world_commands.rs
git commit -m "sim/world_commands: clear c4_plant on Move/Stop"
```

---

### Task 8: Implement `tick_c4_plants` in `world_orders.rs`

**Why:** This is the heart of the feature — walk-up adjacency detection + plant
claim (Phase 1) and per-tick detonation (Phase 2).

**Files:**
- Modify: `src/sim/world/world_orders.rs`

**Pattern:** Mirror `tick_capture_orders` at lines 151-209.

**Step 1: Add the function.**

Insert immediately after `tick_capture_orders` (line 209). Add this function in full:

```rust
    /// Tick C4 plant orders.
    ///
    /// Phase 1 (walk-up): for each entity with `c4_plant`, check if it's on the
    /// target building's cell; if so and the building doesn't already have a
    /// `pending_c4_detonation` claimed by another attacker, claim it. Second
    /// attackers on an already-claimed target hover (no-op) — matches gamemd's
    /// `+0x6df` marker check.
    ///
    /// Phase 2 (detonation): for each building with `pending_c4_detonation`,
    /// if the elapsed tick count >= `rules.c4_delay_ticks`, apply C4Warhead
    /// damage equal to the building's current HP. The pending state is NOT
    /// cleared (gamemd parity OQ2): if the damage is nullified (IronCurtain),
    /// it fires again next tick. When the building dies, the entity despawns
    /// and the pending state goes with it.
    ///
    /// Returns true if any building was destroyed this tick (signals atlas /
    /// owner-count refresh upstream).
    ///
    /// Verified vs gamemd via Ghidra `BuildingClass::Update @ 0x0043fb20` and
    /// `Mission_Enter @ 0x005196a0` (live during 2026-05-10 brainstorm).
    pub(crate) fn tick_c4_plants(&mut self, rules: &RuleSet) -> bool {
        use crate::sim::components::PendingC4Detonation;
        let mut destroyed_structure = false;

        // ---- Phase 1: walk-up + plant claim ----
        // Snapshot attackers with c4_plant. Deterministic sorted order.
        let walkup: Vec<(u64, u64)> = self
            .entities
            .values()
            .filter(|e| e.c4_plant.is_some() && !e.dying)
            .map(|e| (e.stable_id, e.c4_plant.unwrap().target_building_id))
            .collect();

        for (attacker_id, target_id) in walkup {
            // Target gone or dying? Clear c4_plant.
            let target_alive = self
                .entities
                .get(target_id)
                .is_some_and(|b| b.category == EntityCategory::Structure && !b.dying);
            if !target_alive {
                if let Some(e) = self.entities.get_mut(attacker_id) {
                    e.c4_plant = None;
                }
                continue;
            }

            // Adjacent to the target (Chebyshev distance ≤ 1)?
            //
            // gamemd has the SEAL walk INTO the building's cell, but our
            // pathfinder treats building footprints as blocked, so exact-cell
            // match would never trigger. Engineer-capture has the same
            // constraint and uses Chebyshev-≤-1 ([world_orders.rs:189]); we
            // do the same. Observable parity drift: SEAL stands one cell next
            // to the building rather than inside it during the plant
            // animation. Audio, animation, detonation effect, and SEAL
            // survival are unchanged. Documented in the design's parity
            // ledger.
            let attacker_cell = self
                .entities
                .get(attacker_id)
                .map(|e| (e.position.rx, e.position.ry));
            let target_cell = self
                .entities
                .get(target_id)
                .map(|b| (b.position.rx, b.position.ry));
            let adjacent_to_target = match (attacker_cell, target_cell) {
                (Some((arx, ary)), Some((trx, try_))) => {
                    let dx = (arx as i32 - trx as i32).abs();
                    let dy = (ary as i32 - try_ as i32).abs();
                    dx <= 1 && dy <= 1
                }
                _ => false,
            };
            if !adjacent_to_target {
                continue; // walk-up still in progress; movement layer handles it
            }

            // Already claimed by another attacker?
            let already_claimed = self
                .entities
                .get(target_id)
                .is_some_and(|b| b.pending_c4_detonation.is_some());
            if already_claimed {
                // Second SEAL — hover, no-op. Matches gamemd's marker-set early-return.
                continue;
            }

            // Claim the plant.
            if let Some(b) = self.entities.get_mut(target_id) {
                b.pending_c4_detonation = Some(PendingC4Detonation {
                    plant_start_tick: self.tick,
                    attacker_id,
                });
            }

            // Drive the plant animation (FireUp = Attack sequence).
            if let Some(a) = self.entities.get_mut(attacker_id) {
                if let Some(ref mut anim) = a.animation {
                    anim.switch_to(crate::sim::animation::SequenceKind::Attack);
                }
            }

            // SealPlaceBomb spatial sound is queued via SimSoundEvent::C4Planted
            // — variant added in Task 8a (follow-up). No code here yet; see Task 8a.
        }

        // ---- Phase 2: detonation ----
        let det_keys: Vec<u64> = self
            .entities
            .values()
            .filter(|e| e.pending_c4_detonation.is_some() && !e.dying)
            .map(|e| e.stable_id)
            .collect();

        let c4_warhead_id = rules.c4_warhead_id();
        let delay = rules.c4_delay_ticks as u64;

        for building_id in det_keys {
            let pending = self
                .entities
                .get(building_id)
                .and_then(|e| e.pending_c4_detonation);
            let Some(pending) = pending else { continue };

            if self.tick.saturating_sub(pending.plant_start_tick) < delay {
                continue;
            }

            // Timer elapsed — apply C4Warhead damage. Damage value = current_hp
            // for guaranteed one-shot kill (matches gamemd's
            // `&iStack_28 = this->Health` argument to TakeDamage).
            // Pending state NOT cleared on purpose (gamemd parity).
            let dmg: i32 = self
                .entities
                .get(building_id)
                .map(|b| b.health.current as i32)
                .unwrap_or(0);
            if dmg <= 0 {
                continue;
            }

            // Resolve kill-credit. Attacker may have despawned — fall back to None.
            let attacker_for_credit: Option<u64> = self
                .entities
                .get(pending.attacker_id)
                .map(|_| pending.attacker_id);

            // Apply damage. Use whatever the sim's standard "apply warhead damage
            // to one entity" helper is — search combat module for the function
            // that already handles Verses + InfDeath + invulnerability. The
            // engineer-capture sibling does owner mutation directly; for damage,
            // we need the combat path. Candidates (confirm at impl-time):
            //   combat::deal_damage_to(entity_id, damage, warhead_id, attacker_id)
            //   combat::apply_warhead_damage_to_entity(...)
            // If no single-entity helper exists, inline the minimal version:
            //   1. Look up warhead by id from rules
            //   2. Get target's armor index, multiply by warhead.verses[idx]/100
            //   3. Subtract from target.health.current (saturating)
            //   4. If new health == 0, set dying = true
            //   5. Respect invulnerability (is_invulnerable check)
            //   6. Push damage event for downstream combat-effects handler
            let killed = self.apply_c4_damage_to_building(
                building_id,
                dmg as u16,
                c4_warhead_id,
                attacker_for_credit,
                rules,
            );
            if killed {
                destroyed_structure = true;
                // pending_c4_detonation goes away with the entity via despawn path.
                // Trigger scatter walk-away for ANY attacker still on this cell
                // with c4_plant pointing at this building. Matches gamemd
                // Mission_Enter post-detonation block: SetMission(Move) + 1-cell
                // move in a deterministic 1-of-8 direction.
                self.queue_c4_post_detonation_scatter(building_id);
            }
        }

        destroyed_structure
    }

    /// Post-detonation: any attacker that was on the destroyed building's
    /// cell with `c4_plant` targeting this building scatters one cell in a
    /// deterministic direction derived from the current tick. Matches gamemd
    /// `Mission_Enter` (0x005196a0) post-detonation block:
    /// `uVar13 = (tick >> 12 + 1) >> 1 & 7` → 1 of 8 directions via
    /// `g_DirectionDeltaX_Table` / `g_DirectionDeltaY_Table`.
    ///
    /// Also clears each attacker's `c4_plant`.
    fn queue_c4_post_detonation_scatter(&mut self, dead_building_id: u64) {
        // 8 cardinal+ordinal directions (matches gamemd's delta tables).
        // Index 0..7 in standard RA2 order: N, NE, E, SE, S, SW, W, NW.
        const DIR_DELTAS: [(i16, i16); 8] = [
            (0, -1),  // N
            (1, -1),  // NE
            (1, 0),   // E
            (1, 1),   // SE
            (0, 1),   // S
            (-1, 1),  // SW
            (-1, 0),  // W
            (-1, -1), // NW
        ];
        // Mirror gamemd's bit-twiddle: `(tick >> 12 + 1) >> 1 & 7`.
        // Operator precedence in C: `>>` is left-to-right at same level,
        // so this is `(((tick >> 12) + 1) >> 1) & 7`. Use the same.
        let dir: usize = ((((self.tick >> 12) + 1) >> 1) & 7) as usize;
        let (dx, dy) = DIR_DELTAS[dir];

        let bld_cell = self
            .entities
            .get(dead_building_id)
            .map(|b| (b.position.rx, b.position.ry));
        let Some((brx, bry)) = bld_cell else { return };

        // Collect attackers on this cell with c4_plant on this building.
        let scatterers: Vec<u64> = self
            .entities
            .values()
            .filter(|e| {
                !e.dying
                    && e.position.rx == brx
                    && e.position.ry == bry
                    && e.c4_plant
                        .map_or(false, |p| p.target_building_id == dead_building_id)
            })
            .map(|e| e.stable_id)
            .collect();

        for sid in scatterers {
            let target_rx = (brx as i16 + dx).max(0) as u16;
            let target_ry = (bry as i16 + dy).max(0) as u16;
            if let Some(e) = self.entities.get_mut(sid) {
                e.c4_plant = None;
            }
            // Issue a one-cell move to the scatter cell. Use the same helper
            // CaptureBuilding / PlantC4 dispatch use — copy the call site.
            // (Implementation tip: extract the move-issue boilerplate from
            // world_commands.rs:917-948 into a small helper if duplication
            // becomes ugly.)
            // For this task, queue a Command::Move via pending_commands —
            // simpler than reimplementing the pathfind call, and the next
            // tick processes the command. The 1-tick delay is below
            // human-observable threshold.
            // Skip if pending_commands isn't accessible from here; otherwise:
            if let Some(owner) = self.entities.get(sid).map(|e| e.owner) {
                self.pending_commands.push(crate::sim::command::CommandEnvelope::new(
                    owner,
                    self.tick + 1,
                    crate::sim::command::Command::Move {
                        entity_id: sid,
                        target_rx,
                        target_ry,
                        queue: false,
                        group_id: None,
                    },
                ));
            }
        }
    }

    /// Apply one C4Warhead damage instance to a building entity. Returns true
    /// if the building died this call. Honors IronCurtain via the standard
    /// invulnerability check. Used by `tick_c4_plants` Phase 2.
    ///
    /// This is a thin wrapper around whatever damage-application helper the
    /// combat module exposes. If `combat::apply_damage_to_entity` doesn't
    /// exist, this function inlines the minimum.
    fn apply_c4_damage_to_building(
        &mut self,
        building_id: u64,
        damage: u16,
        warhead_id: crate::sim::intern::InternedId,
        attacker_id: Option<u64>,
        rules: &RuleSet,
    ) -> bool {
        // Check IC — if invulnerable, damage is nullified but we don't clear
        // pending_c4_detonation, so we'll try again next tick.
        let invuln = self
            .entities
            .get(building_id)
            .and_then(|e| e.invulnerability)
            .as_ref()
            .copied();
        if crate::sim::superweapon::invulnerability::is_invulnerable(
            invuln.as_ref(),
            self.tick as u32,
        ) {
            return false;
        }

        // Resolve warhead, apply Verses, subtract HP.
        let warhead_name = self.interner.resolve(warhead_id);
        let Some(warhead) = rules.warhead(warhead_name) else {
            return false;
        };
        let (armor_idx, max_hp) = match self.entities.get(building_id) {
            Some(b) => {
                let obj = rules
                    .object(self.interner.resolve(b.type_ref))
                    .map(|o| o.armor.as_str())
                    .unwrap_or("none");
                (crate::sim::combat::armor_index(obj), b.health.max)
            }
            None => return false,
        };
        let verses_pct = warhead.verses.get(armor_idx).copied().unwrap_or(100);
        let scaled = (damage as i32 * verses_pct as i32 / 100).max(0) as u16;

        let killed = {
            let Some(b) = self.entities.get_mut(building_id) else { return false };
            let new_hp = b.health.current.saturating_sub(scaled);
            b.health.current = new_hp;
            if new_hp == 0 {
                b.dying = true;
                if let Some(att) = attacker_id {
                    b.last_attacker_id = Some(att);
                }
                true
            } else {
                false
            }
        };
        // Honor warhead InfDeath for any infantry on the cell (deferred — stock
        // maps don't stack infantry on building cells; covered in Task 13 #11).
        killed
    }
```

**Step 2: Verify the function compiles.**

```
cargo build -p ra2_rust
```

If `apply_damage_to_entity`-style helper already exists in `sim/combat`, replace the
inlined block with a single call. Search for it: `grep -n "fn apply_damage" src/sim/combat/`.

**Step 3: Commit (build-only; tick integration in Task 9, behavioral tests in Task 13).**

```
git add src/sim/world/world_orders.rs
git commit -m "sim/world_orders: add tick_c4_plants (walk-up + detonation)"
```

---

### Task 8a: Wire `SimSoundEvent::C4Planted` for the SealPlaceBomb cue

**Why:** Audible plant-claim feedback is a parity-relevant observable. The existing
`SimSoundEvent` enum has specialized variants per cue (WeaponFired, EntityDeployed,
ChuteSound...); none generically map to "play [SealPlaceBomb]." Adding one dedicated
variant + app handler is the lowest-touch path. Split from Task 8 because it spans
sim → app and warrants a separate commit.

**Files:**
- Modify: `src/sim/world/mod.rs` (add `SimSoundEvent::C4Planted` variant)
- Modify: `src/sim/world/world_orders.rs` (emit the event from Task 8's Phase 1 claim point)
- Modify: the app-side audio dispatcher that drains `sim.sound_events` (search for
  `SimSoundEvent::ChuteSound` consumer to find the right file)

**Step 1: Add the variant** to `SimSoundEvent` in `src/sim/world/mod.rs` near
`ChuteSound` (line 163):

```rust
    /// A C4-capable infantry claimed a plant on a CanC4 building.
    /// Played at the attacker's position. App resolves to
    /// `[SealPlaceBomb]` in soundmd.ini.
    C4Planted { rx: u16, ry: u16 },
```

**Step 2: Emit from `tick_c4_plants` Phase 1 plant-claim block** (the spot that
currently says "see Task 8a"):

```rust
            if let Some(a) = self.entities.get(attacker_id) {
                self.sound_events.push(crate::sim::world::SimSoundEvent::C4Planted {
                    rx: a.position.rx,
                    ry: a.position.ry,
                });
            }
```

**Step 3: Add the app-side handler.** Find the file that matches existing variants
to soundmd cues. Search:

```
grep -rn "SimSoundEvent::ChuteSound" src/ | grep -v "/sim/"
```

That gives the app-side dispatcher. Add a match arm:

```rust
            SimSoundEvent::C4Planted { rx, ry } => {
                play_spatial_sound(state, "SealPlaceBomb", *rx, *ry);
            }
```

(Use the function name used by the surrounding arms — likely `play_spatial_sound`,
`enqueue_voc`, or similar.)

**Step 4: Verify.**

```
cargo build -p ra2_rust
```

Manual playtest: hear the sound during plant claim (covered by Task 16 #5).

**Step 5: Commit.**

```
git add src/sim/world/mod.rs src/sim/world/world_orders.rs <app-side-file>
git commit -m "sim+app: emit SealPlaceBomb sound on C4 plant claim"
```

---

### Task 9: Wire `tick_c4_plants` into `advance_tick`

**Why:** Without a call site, the handler is dead code.

**Files:**
- Modify: `src/sim/world/mod.rs`

**Pattern:** Slot immediately after `tick_capture_orders` at line 1174.

**Step 1: Insert the call.**

At line 1174 in `src/sim/world/mod.rs`:

```rust
            spawned_entities |= self.tick_capture_orders();
            destroyed_structure |= self.tick_c4_plants(rules);   // NEW
            self.tick_order_intents_pre_combat(rules);
```

**Step 2: Update the surrounding doc comment** (the block comment at lines 1167-1173
describes the Phase 5 ordering). Append one sentence after "PRODUCES: damage, deaths,
...":

```rust
            // --- Phase 5: Combat + Turret rotation ---
            // DEPENDS ON: vision/fog (targeting uses fog state), power (cloaking).
            // ...
            // tick_c4_plants runs alongside tick_capture_orders — both convert
            // walk-up intent into a state change on arrival. Detonation damage
            // is applied here so combat-pre conditions (invulnerability, dying)
            // are honored before tick_combat runs.
            // PRODUCES: damage, deaths, bridge damage, fire events, last_attacker_id.
```

**Step 3: Verify build + run existing tests.**

```
cargo build -p ra2_rust
cargo test -p ra2_rust --lib sim::world -- --nocapture
```

Expected: clean build, no regressions in existing world tests.

**Step 4: Commit.**

```
git add src/sim/world/mod.rs
git commit -m "sim/world: wire tick_c4_plants into advance_tick"
```

---

### Task 10: Add `CursorFeedbackKind::Demolish` and feedback→cursor mapping

**Why:** The cursor visual layer needs a new feedback variant. `CursorId::Demolish`
already exists and is already loaded; only the kind enum and mapping are missing.

**Files:**
- Modify: `src/app_types.rs`
- Modify: `src/app_cursor.rs`

**Pattern:** Mirror `Enter`/`EngineerRepair` variants in `CursorFeedbackKind` and the
matching arm in `cursor_id_for_feedback`.

**Step 1: Add the variant.**

In `src/app_types.rs`, locate the `CursorFeedbackKind` enum (line 157). Add a new
variant after `EngineerRepair`:

```rust
    /// Engineer repair cursor — engineer hovering a damaged friendly building.
    EngineerRepair,
    /// C4 plant cursor — SEAL/Tanya/PTROOP hovering a CanC4 enemy structure
    /// (action 0x10 in gamemd, distinct mouse.shp frames from Enter).
    Demolish,
```

**Step 2: Map the new feedback kind to its CursorId.**

In `src/app_cursor.rs` at line 457 (`CursorFeedbackKind::Enter => Some(CursorId::Enter),`),
add right after:

```rust
        CursorFeedbackKind::EngineerRepair => Some(CursorId::EngineerRepair),
        CursorFeedbackKind::Demolish => Some(CursorId::Demolish),
```

(The EngineerRepair line is already there; just add the Demolish arm.)

**Step 3: Verify.**

```
cargo build -p ra2_rust
```

**Step 4: Commit.**

```
git add src/app_types.rs src/app_cursor.rs
git commit -m "app/cursor: add CursorFeedbackKind::Demolish + CursorId mapping"
```

---

### Task 11: Gate cursor-feedback branch on `c4 && can_c4 && !invisible_in_game`

**Why:** Currently the SabotageCursor branch shows `Enter` for any SabotageCursor weapon
hovering any enemy structure (ungated). gamemd's action 0x10 (Demolish) requires
attacker `C4=yes` AND target `CanC4=yes` AND not invisible AND not iron-curtained.

**Files:**
- Modify: `src/app_cursor.rs`

**Pattern:** Mirror the engineer cursor branch (lines 223-238).

**Step 1: Replace the existing `sabotage_cursor` branch.**

In `src/app_cursor.rs` at lines 214-219, replace:

```rust
            // 2. SabotageCursor: Tanya/Navy SEAL hovering an enemy structure.
            if sel_obj.sabotage_cursor {
                if matches!(hover.kind, HoverTargetKind::EnemyStructure) {
                    return CursorFeedbackKind::Enter;
                }
            }
```

with:

```rust
            // 2. C4 plant: SEAL / Tanya / Psi-Corp Trooper hovering an enemy
            //    structure with CanC4=yes, not InvisibleInGame, not iron-curtained.
            //    SabotageCursor flag remains in the data model (parsed in
            //    object_type.rs) for modder weapon-overlay use, but cursor
            //    logic is now driven by C4=yes — matches gamemd action 0x10.
            if sel_obj.c4
                && matches!(hover.kind, HoverTargetKind::EnemyStructure)
                && hovered_obj.map_or(false, |o| o.can_c4 && !o.invisible_in_game)
                && !hovered_entity.is_some_and(|e| {
                    crate::sim::superweapon::invulnerability::is_invulnerable(
                        e.invulnerability.as_ref(),
                        sim.tick as u32,
                    )
                })
            {
                return CursorFeedbackKind::Demolish;
            }
```

**Step 2: Verify build + run existing cursor tests.**

```
cargo build -p ra2_rust
cargo test -p ra2_rust --lib app_cursor -- --nocapture
```

Expected: clean build. Existing tests should still pass since stock SEAL/Tanya have
`SabotageCursor=yes` AND will have `C4=yes` after Task 1, so the cursor still triggers
— just routes to Demolish instead of Enter.

**Step 3: Commit.**

```
git add src/app_cursor.rs
git commit -m "app/cursor: gate sabotage cursor on c4 + can_c4 + IC, route to Demolish"
```

---

### Task 12: Emit `Command::PlantC4` from right-click in `app_context_order.rs`

**Why:** Without this, the right-click still falls through to other order paths. Must
be ordered BEFORE the engineer branch (line 311) so C4-capable units take the plant
path even if some hypothetical modder unit had both `engineer` and `c4` set.

**Files:**
- Modify: `src/app_context_order.rs`

**Pattern:** Mirror the engineer-capture branch (lines 311-367).

**Step 1: Insert the C4 plant branch immediately before line 311.**

```rust
            // C4 plant: SEAL / Tanya / Psi-Corp Trooper clicking a CanC4 enemy
            // structure. Ordered before the engineer-capture branch so C4 takes
            // priority for any unit with both flags.
            if !force_fire {
                let c4_target = hover.as_ref().and_then(|target| {
                    if !matches!(target.kind, HoverTargetKind::EnemyStructure) {
                        return None;
                    }
                    let rules = state.rules.as_ref()?;
                    let building = sim.entities.get(target.stable_id)?;
                    let obj = rules.object(sim.interner.resolve(building.type_ref))?;
                    if !obj.can_c4 || obj.invisible_in_game {
                        return None;
                    }
                    // Reject IC'd target at issue time (matches gamemd's
                    // What_Action_OnObject vtable[+0x80] check).
                    if crate::sim::superweapon::invulnerability::is_invulnerable(
                        building.invulnerability.as_ref(),
                        sim.tick as u32,
                    ) {
                        return None;
                    }
                    Some(target.stable_id)
                });
                if let Some(building_id) = c4_target {
                    let c4_attackers: Vec<u64> = selected_units
                        .iter()
                        .copied()
                        .filter(|&sid| {
                            sim.entities.get(sid).is_some_and(|e| {
                                e.category == EntityCategory::Infantry
                                    && state
                                        .rules
                                        .as_ref()
                                        .and_then(|r| r.object(sim.interner.resolve(e.type_ref)))
                                        .map_or(false, |o| o.c4)
                            })
                        })
                        .collect();
                    if !c4_attackers.is_empty() {
                        for attacker_id in c4_attackers {
                            queued.push(CommandEnvelope::new(
                                owner_id,
                                execute_tick,
                                Command::PlantC4 {
                                    attacker_id,
                                    target_building_id: building_id,
                                },
                            ));
                        }
                        for cmd in queued {
                            sim.pending_commands.push(cmd);
                        }
                        // EVA voice for the plant order. Matches gamemd's
                        // VoiceSpecialAttack=SealSpecialAttack on [GHOST].
                        emit_order_voice(state, "VoiceSpecialAttack");
                        return true;
                    }
                }
            }
```

**Step 2: Verify build.**

```
cargo build -p ra2_rust
```

Expected: clean build. Behavioral end-to-end coverage in Task 13.

**Step 3: Commit.**

```
git add src/app_context_order.rs
git commit -m "app/context_order: route SEAL/Tanya right-click on CanC4 building to PlantC4"
```

---

### Task 13: Integration tests for the C4 lifecycle

**Why:** Eight scenarios cover the parity-critical behaviors (happy path, attacker
death, IC, two SEALs, target death, Stop, gating). One scenario per test.

**Files:**
- Modify: `src/sim/world/world_tests.rs` (append) OR Create: `src/sim/world/world_orders_c4_tests.rs`

Use whichever follows the project's convention — search for a sibling test file in
`src/sim/world/`. If world tests are aggregated in `world_tests.rs`, append there.

**Pattern:** Mirror the existing capture / bridge tests in `world_tests.rs`. They
typically build a sim with `build_minimal_sim`, spawn entities, advance ticks via
`sim.advance_tick(...)`, then assert state.

**Verified advance_tick signature** (from [world/mod.rs:970-978](../../src/sim/world/mod.rs#L970-L978)):

```rust
pub fn advance_tick(
    &mut self,
    commands: &[CommandEnvelope],
    rules: Option<&RuleSet>,
    height_map: &BTreeMap<(u16, u16), u8>,
    path_grid: Option<&PathGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    tick_ms: u32,
) -> TickResult
```

Existing test convention (from `world_tests.rs:1346`):
```rust
sim.advance_tick(&[], Some(&rules), &heights, Some(&path_grid), None, 67);
```

**Verified queue/dispatch helpers:**
- `sim.queue_command(cmd: CommandEnvelope)` — single arg, an envelope
  ([world/mod.rs:418](../../src/sim/world/mod.rs#L418)).
- **No `dispatch_command(owner, &Command)` method exists.** To test
  "command is rejected at issue time" — queue + advance_tick + assert that
  the attacker's state didn't change.

Each test sets up `let rules = ...; let heights = empty_heights(); let path_grid = ...;`
matching the existing convention. If `build_sim_with_c4_rules`, `spawn_infantry`,
`spawn_building` helpers don't exist, build them on top of `build_minimal_sim`.
**Sub-task 13.0: write the test helpers if they don't already exist** — search
nearby tests for what's there before duplicating. The helper MUST call
`rules.resolve_bridge_warheads(&mut sim.interner)` (verified existing pattern at
[world_tests.rs:523](../../src/sim/world/world_tests.rs#L523)) because Task 8's
detonation tick calls `rules.c4_warhead_id()`, which panics if resolution hasn't run.

**Step 1: Add the test module skeleton (if creating a new file)** with the standard
test harness imports. Match the imports at the top of `world_tests.rs`.

**Step 2: Test 1 — Happy path: SEAL plants, 27 ticks elapse, building dies, SEAL survives.**

```rust
#[test]
fn c4_plant_happy_path_kills_building_and_seal_survives() {
    let mut sim = build_sim_with_c4_rules();
    // Spawn a SEAL at (5,5) and an Allied Barracks at (10,10).
    let seal = spawn_infantry(&mut sim, "GHOST", "Americans", 5, 5);
    let bld = spawn_building(&mut sim, "GAPILE", "Soviets", 10, 10);

    // Issue PlantC4 order from Americans on the enemy Soviet building.
    let owner = sim.interner.intern("Americans");
    sim.queue_command(CommandEnvelope::new(
        owner,
        sim.tick + 1,
        Command::PlantC4 {
            attacker_id: seal,
            target_building_id: bld,
        },
    ));

    // Tick until SEAL is adjacent (Chebyshev ≤ 1) to the building. Walk-up
    // takes some ticks depending on pathfinding; cap at 200.
    for _ in 0..200 {
        sim.advance_tick(&[], Some(&rules), &heights, Some(&path_grid), None, 67);
        if let Some(pos) = sim.entities.get(seal).map(|e| (e.position.rx, e.position.ry)) {
            let dx = (pos.0 as i32 - 10).abs();
            let dy = (pos.1 as i32 - 10).abs();
            if dx <= 1 && dy <= 1 { break; }
        }
    }
    let pos = sim.entities.get(seal).map(|e| (e.position.rx, e.position.ry)).unwrap();
    let dx = (pos.0 as i32 - 10).abs();
    let dy = (pos.1 as i32 - 10).abs();
    assert!(dx <= 1 && dy <= 1, "SEAL must become adjacent to target within 200 ticks");

    // Adjacent-tick: plant is claimed.
    assert!(sim.entities.get(bld).unwrap().pending_c4_detonation.is_some(),
            "plant must be claimed on adjacency");
    let plant_start = sim.entities.get(bld).unwrap().pending_c4_detonation.unwrap().plant_start_tick;

    // Advance c4_delay_ticks (27). Building must die on tick plant_start + 27.
    let delay = rules.c4_delay_ticks as u64;
    for _ in 0..delay {
        sim.advance_tick(&[], Some(&rules), &heights, Some(&path_grid), None, 67);
    }
    // One more tick for the detonation phase to fire.
    sim.advance_tick(/* ... */);

    // Building dies.
    assert!(
        sim.entities.get(bld).map_or(true, |b| b.dying || b.health.current == 0),
        "building must be destroyed at plant_start + c4_delay"
    );
    // SEAL survives.
    assert!(sim.entities.get(seal).is_some(), "SEAL must survive the plant");
    assert!(!sim.entities.get(seal).unwrap().dying, "SEAL must not be dying");
}
```

**Step 3: Test 2 — Attacker dies mid-plant: detonation still fires (OQ2 verified).**

```rust
#[test]
fn c4_attacker_death_does_not_abort_detonation() {
    let mut sim = build_sim_with_c4_rules();
    let seal = spawn_infantry(&mut sim, "GHOST", "Americans", 10, 10);
    let bld = spawn_building(&mut sim, "GAPILE", "Soviets", 10, 10);

    // Manually claim the plant (skip walk-up for test clarity).
    sim.entities.get_mut(bld).unwrap().pending_c4_detonation =
        Some(PendingC4Detonation { plant_start_tick: sim.tick, attacker_id: seal });

    // Mid-plant: kill the SEAL (5 ticks in).
    for _ in 0..5 { sim.advance_tick(/* ... */); }
    sim.entities.get_mut(seal).unwrap().health.current = 0;
    sim.entities.get_mut(seal).unwrap().dying = true;
    // Let combat tick remove it.
    sim.advance_tick(/* ... */);
    assert!(
        sim.entities.get(seal).is_none() || sim.entities.get(seal).unwrap().dying,
        "SEAL must be despawned or dying after kill"
    );

    // Advance through C4Delay. Building MUST still die.
    let delay = rules.c4_delay_ticks as u64;
    for _ in 0..(delay + 2) { sim.advance_tick(/* ... */); }
    assert!(
        sim.entities.get(bld).map_or(true, |b| b.dying || b.health.current == 0),
        "PARITY (OQ2): detonation must fire even after attacker death"
    );
}
```

**Step 4: Test 3 — IC during plant: damage retries every tick until IC expires.**

```rust
#[test]
fn c4_iron_curtain_blocks_until_expiry_then_kills() {
    use crate::sim::superweapon::invulnerability::{InvulnerabilityState, InvulnKind};
    let mut sim = build_sim_with_c4_rules();
    let seal = spawn_infantry(&mut sim, "GHOST", "Americans", 10, 10);
    let bld = spawn_building(&mut sim, "GAPILE", "Soviets", 10, 10);

    // Claim the plant, then immediately IC the building.
    sim.entities.get_mut(bld).unwrap().pending_c4_detonation =
        Some(PendingC4Detonation { plant_start_tick: sim.tick, attacker_id: seal });
    sim.entities.get_mut(bld).unwrap().invulnerability = Some(InvulnerabilityState {
        start_frame: sim.tick as u32,
        duration_frames: 40, // outlasts C4Delay (27)
        kind: InvulnKind::IronCurtain,
    });

    // Advance through C4Delay + 5. Building must STILL be alive (IC nullifies).
    let delay = rules.c4_delay_ticks as u64;
    for _ in 0..(delay + 5) { sim.advance_tick(/* ... */); }
    assert!(
        sim.entities.get(bld).is_some_and(|b| !b.dying && b.health.current > 0),
        "IC must block C4 damage while active"
    );

    // Advance past IC duration. Next damage tick kills the building.
    for _ in 0..40 { sim.advance_tick(/* ... */); }
    assert!(
        sim.entities.get(bld).map_or(true, |b| b.dying || b.health.current == 0),
        "PARITY: building must die after IC expires (damage retries every tick)"
    );
}
```

**Step 5: Test 4 — Two SEALs same target: second hovers, no second plant.**

```rust
#[test]
fn second_c4_attacker_does_not_overwrite_plant() {
    let mut sim = build_sim_with_c4_rules();
    let seal_a = spawn_infantry(&mut sim, "GHOST", "Americans", 10, 10);
    let seal_b = spawn_infantry(&mut sim, "TANY", "Americans", 10, 10);
    let bld = spawn_building(&mut sim, "GAPILE", "Soviets", 10, 10);

    // A plants first.
    sim.entities.get_mut(seal_a).unwrap().c4_plant =
        Some(C4PlantState { target_building_id: bld });
    sim.entities.get_mut(seal_b).unwrap().c4_plant =
        Some(C4PlantState { target_building_id: bld });

    // First tick: A claims. B sees claim and hovers.
    sim.advance_tick(/* ... */);
    let pending = sim.entities.get(bld).unwrap().pending_c4_detonation.unwrap();
    assert_eq!(pending.attacker_id, seal_a,
               "first attacker wins the claim (deterministic by sorted iteration)");

    // A's c4_plant stays (it's on-cell with the building). B's c4_plant also stays
    // (it's on-cell but the building is already claimed). Neither overwrites pending.
    sim.advance_tick(/* ... */);
    let pending_after = sim.entities.get(bld).unwrap().pending_c4_detonation.unwrap();
    assert_eq!(pending_after.plant_start_tick, pending.plant_start_tick,
               "pending plant_start_tick must not be overwritten by second attacker");
    assert_eq!(pending_after.attacker_id, seal_a,
               "pending attacker must not be overwritten by second attacker");
}
```

**Step 6: Test 5 — Target dies before timer: attacker's c4_plant clears.**

```rust
#[test]
fn target_death_clears_c4_plant_on_attacker() {
    let mut sim = build_sim_with_c4_rules();
    let seal = spawn_infantry(&mut sim, "GHOST", "Americans", 5, 5);
    let bld = spawn_building(&mut sim, "GAPILE", "Soviets", 10, 10);

    sim.entities.get_mut(seal).unwrap().c4_plant =
        Some(C4PlantState { target_building_id: bld });

    // Kill the building via direct mutation (simulate another weapon).
    sim.entities.get_mut(bld).unwrap().health.current = 0;
    sim.entities.get_mut(bld).unwrap().dying = true;

    sim.advance_tick(/* ... */);

    // After tick_c4_plants Phase 1 sees target gone, c4_plant clears.
    assert!(
        sim.entities.get(seal).unwrap().c4_plant.is_none(),
        "c4_plant must clear when target dies"
    );
}
```

**Step 7: Test 6 — Stop cancels walk-up; plant_already_claimed survives Stop.**

```rust
#[test]
fn stop_cancels_walkup_but_not_already_claimed_plant() {
    let mut sim = build_sim_with_c4_rules();
    let seal = spawn_infantry(&mut sim, "GHOST", "Americans", 5, 5);
    let bld = spawn_building(&mut sim, "GAPILE", "Soviets", 10, 10);

    let owner = sim.interner.intern("Americans");

    // Case A: Stop DURING walk-up clears c4_plant.
    sim.entities.get_mut(seal).unwrap().c4_plant =
        Some(C4PlantState { target_building_id: bld });
    sim.queue_command(CommandEnvelope::new(owner, sim.tick + 1,
        Command::Stop { entity_id: seal }));
    sim.advance_tick(&[], Some(&rules), &heights, Some(&path_grid), None, 67);
    assert!(sim.entities.get(seal).unwrap().c4_plant.is_none(),
            "Stop must clear c4_plant during walk-up");
    assert!(sim.entities.get(bld).unwrap().pending_c4_detonation.is_none(),
            "no plant was claimed, building stays clean");

    // Case B: Stop AFTER plant is claimed does NOT clear pending_c4_detonation.
    sim.entities.get_mut(bld).unwrap().pending_c4_detonation =
        Some(PendingC4Detonation { plant_start_tick: sim.tick, attacker_id: seal });
    sim.queue_command(CommandEnvelope::new(owner, sim.tick + 1,
        Command::Stop { entity_id: seal }));
    sim.advance_tick(&[], Some(&rules), &heights, Some(&path_grid), None, 67);
    assert!(sim.entities.get(bld).unwrap().pending_c4_detonation.is_some(),
            "PARITY: claimed plant survives Stop on attacker");

    // And the building still detonates on schedule.
    let delay = rules.c4_delay_ticks as u64;
    for _ in 0..(delay + 2) {
        sim.advance_tick(&[], Some(&rules), &heights, Some(&path_grid), None, 67);
    }
    assert!(
        sim.entities.get(bld).map_or(true, |b| b.dying || b.health.current == 0),
        "claimed plant detonates on schedule even after Stop on attacker"
    );
}
```

**Step 8: Test 7 — `CanC4=no` building rejects PlantC4 at issue time.**

```rust
#[test]
fn cannot_c4_building_rejects_plant_command() {
    let mut sim = build_sim_with_c4_rules();
    let seal = spawn_infantry(&mut sim, "GHOST", "Americans", 5, 5);
    let oil = spawn_building(&mut sim, "CAMISC01", "Soviets", 10, 10); // Oil Derrick

    let owner = sim.interner.intern("Americans");
    // No public `dispatch_command` helper — queue + advance and assert the
    // command had no effect (silently rejected by Task 6's validation block).
    sim.queue_command(CommandEnvelope::new(owner, sim.tick + 1, Command::PlantC4 {
        attacker_id: seal,
        target_building_id: oil,
    }));
    sim.advance_tick(&[], Some(&rules), &heights, Some(&path_grid), None, 67);
    assert!(
        sim.entities.get(seal).unwrap().c4_plant.is_none(),
        "PlantC4 must be silently rejected for CanC4=no buildings"
    );
    assert!(
        sim.entities.get(oil).unwrap().pending_c4_detonation.is_none(),
        "rejected PlantC4 must not set pending_c4_detonation on the target"
    );
}
```

**Step 9: Test 8 — Non-C4 unit rejects PlantC4 at issue time.**

```rust
#[test]
fn non_c4_unit_rejects_plant_command() {
    let mut sim = build_sim_with_c4_rules();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 5, 5);
    let bld = spawn_building(&mut sim, "GAPILE", "Soviets", 10, 10);

    let owner = sim.interner.intern("Americans");
    sim.queue_command(CommandEnvelope::new(owner, sim.tick + 1, Command::PlantC4 {
        attacker_id: gi,
        target_building_id: bld,
    }));
    sim.advance_tick(&[], Some(&rules), &heights, Some(&path_grid), None, 67);
    assert!(
        sim.entities.get(gi).unwrap().c4_plant.is_none(),
        "PlantC4 must be silently rejected for non-C4 attackers"
    );
    assert!(
        sim.entities.get(bld).unwrap().pending_c4_detonation.is_none(),
        "rejected PlantC4 must not set pending_c4_detonation on the target"
    );
}
```

**Step 10: Verify.**

```
cargo test -p ra2_rust --lib c4 -- --nocapture
```

Expected: all 8 tests pass.

**Step 11: Commit.**

```
git add src/sim/world/world_tests.rs   # or world_orders_c4_tests.rs
git commit -m "sim/world: integration tests for C4 plant lifecycle (8 cases)"
```

---

### Task 14: Determinism / replay regression test

**Why:** Lockstep correctness — running the same C4 scenario from the same initial
state with the same commands must produce identical state hashes at every tick.

**Files:**
- Modify: `src/sim/world/world_tests.rs` (append)

**Pattern:** Mirror existing determinism / replay tests if any (search the file).
If none exist, the pattern is: run the scenario twice on independent sim instances,
hash at every tick, compare.

**Step 1: Add the test.**

```rust
#[test]
fn c4_lifecycle_is_deterministic() {
    fn run() -> Vec<u64> {
        let mut sim = build_sim_with_c4_rules();
        let seal = spawn_infantry(&mut sim, "GHOST", "Americans", 5, 5);
        let bld = spawn_building(&mut sim, "GAPILE", "Soviets", 10, 10);
        let owner = sim.interner.intern("Americans");
        sim.queue_command(CommandEnvelope::new(owner, sim.tick + 1, Command::PlantC4 {
            attacker_id: seal,
            target_building_id: bld,
        }));
        let mut hashes = Vec::new();
        for _ in 0..100 {
            sim.advance_tick(&[], Some(&rules), &heights, Some(&path_grid), None, 67);
            hashes.push(sim.state_hash());
        }
        hashes
    }

    let h1 = run();
    let h2 = run();
    assert_eq!(h1, h2, "C4 lifecycle must be deterministic across runs");
}
```

**Step 2: Verify.**

```
cargo test -p ra2_rust --lib c4_lifecycle_is_deterministic -- --nocapture
```

**Step 3: Commit.**

```
git add src/sim/world/world_tests.rs
git commit -m "sim/world: determinism test for C4 lifecycle"
```

---

### Task 15: INI parse verification (stock units)

**Why:** Sanity-check that retail INI values flow through to the right flags.

**Files:**
- Modify: `src/rules/ruleset.rs` (append integration test) OR `tests/ini_parse.rs`

**Step 1: Add the verification test.**

```rust
#[test]
fn retail_rulesmd_c4_flags_parse_correctly() {
    // Load the actual retail rulesmd.ini from the repo's ini/ directory.
    let ini_text = std::fs::read_to_string("ini/rulesmd.ini").expect("ini/rulesmd.ini");
    let ini = IniFile::from_str(&ini_text);
    let rules = RuleSet::from_ini(&ini).expect("parse retail rulesmd");

    // C4-capable units must have c4=true.
    for unit in &["GHOST", "TANY", "PTROOP"] {
        let obj = rules.object(unit).unwrap_or_else(|| panic!("no [{}]", unit));
        assert!(obj.c4, "[{}] must have c4=true (C4=yes in INI)", unit);
    }
    // Non-C4 infantry must have c4=false.
    for unit in &["E1", "ENGINEER", "CCOMAND"] {
        if let Some(obj) = rules.object(unit) {
            assert!(!obj.c4, "[{}] must have c4=false", unit);
        }
    }

    // CanC4-opt-out buildings.
    for bld in &["CAMISC01", "CAMISC02", "CAMSC09", "CAMSC10"] {
        if let Some(obj) = rules.object(bld) {
            assert!(!obj.can_c4, "[{}] must have can_c4=false (CanC4=no in INI)", bld);
        }
    }
    // Normal buildings inherit can_c4=true.
    for bld in &["GAPILE", "NAHAND", "GAREFN"] {
        if let Some(obj) = rules.object(bld) {
            assert!(obj.can_c4, "[{}] must have can_c4=true (default)", bld);
        }
    }

    // C4Delay must match the retail value (0.03 minutes = 27 ticks).
    assert_eq!(rules.c4_delay_ticks, 27, "C4Delay must parse to 27 ticks");
}
```

**Step 2: Verify.**

```
cargo test -p ra2_rust --lib retail_rulesmd_c4_flags_parse_correctly -- --nocapture
```

**Step 3: Commit.**

```
git add src/rules/ruleset.rs
git commit -m "rules/ruleset: verify retail rulesmd.ini C4 flags parse correctly"
```

---

### Task 16: Manual playtest checklist (parity verification)

**Why:** Unit/integration tests cover state transitions but not pixel-level cursor
frames, sound spatialization, or animation visual feel. This task is a manual
end-to-end run.

**Files:** None (verification only).

**Steps to verify in a running game** (Allied skirmish on any map with a SEAL or Tanya):

1. **Cursor: Demolish on enemy barracks.**
   - Build a SEAL or train Tanya. Select it.
   - Hover an enemy Allied Barracks / Soviet Barracks.
   - **Expected:** cursor shows the Demolish frames (animated, 10 frames at standard
     interval) — visually distinct from the Enter cursor used for engineers.
   - **Failure mode:** if cursor shows Enter (single static frame for some sequences,
     or different animation), Task 11's gating is wrong.

2. **Cursor: Attack (no Demolish) on Oil Derrick.**
   - Hover an Oil Derrick (`CAMISC01`).
   - **Expected:** cursor shows Attack / EnemyStructure, NOT Demolish.

3. **Cursor: Attack (no Demolish) on iron-curtained building.**
   - Iron Curtain an enemy building (use sandbox / cheats).
   - Hover with SEAL.
   - **Expected:** Attack cursor, not Demolish.

4. **Plant happy path: walk-up + 27-tick wait + boom.**
   - Right-click SEAL on enemy Barracks.
   - **Expected:** SEAL walks adjacent to the building (Chebyshev-≤-1 — our
     pathfinder blocks infantry from building-footprint cells; documented parity
     drift vs gamemd which has SEAL inside the cell), briefly plays the
     FireUp / Attack animation, building explodes ~1.8 seconds after arrival,
     SEAL survives and walks one cell.

5. **Audio: `SealPlaceBomb` on plant claim.**
   - Listen during step 4 around the plant moment.
   - **Expected:** brief "place bomb" sound effect plays at the building's position.

6. **EVA voice: `SealSpecialAttack` on right-click order.**
   - On the right-click order (step 4 trigger), listen.
   - **Expected:** global EVA voice line plays at command time.

7. **SEAL survives, scatters 1 cell:** confirm step 4 final state — SEAL is alive and
   on an adjacent cell.

8. **Iron Curtain mid-plant.**
   - Start C4 plant. Before detonation tick, IC the building.
   - **Expected:** building survives IC duration, then dies on the next tick after
     IC expires.

9. **Attacker death mid-plant.**
   - Start C4 plant. Before detonation, kill the SEAL with own forces (force-fire).
   - **Expected:** building STILL explodes on schedule.

10. **Two SEALs same target:**
    - Order two SEALs to plant on the same building.
    - **Expected:** the second arrival idles near the building; only one plant
      runs; building dies once.

**If any of these fail, file a regression-test gap and re-open the corresponding
implementation task.**

**Step 11: Commit notes.**

This task itself doesn't produce a commit; if regressions are found, document in
a new commit referencing the failing checklist item.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-10-navy-seal-c4-demolition-design.md](2026-05-10-navy-seal-c4-demolition-design.md)
- **Investigation plan:** docs/plans/2026-05-10-navy-seal-c4-demolition-investigation-plan.md
- **Ghidra reports:**
  - `ra2-rust-game-docs/NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md` (primary, this feature)
  - `ra2-rust-game-docs/BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` (cross-ref)
  - `ra2-rust-game-docs/READINI_FIELD_MAPS.md` (INI key offsets)
  - `ra2-rust-game-docs/MouseClass_research.md` (cursor action 0x10)
- **gamemd.exe addresses verified live in brainstorm:**
  - `0x005196a0` — `InfantryClass::Mission_Enter` (plant claim site + post-detonation block)
  - `0x0043fb20` — `BuildingClass::Update` (detonation timer + damage application, OQ2)
  - `0x00524400` (entry `0x005240a0`) — `InfantryTypeClass::ReadINI` (C4 flag at +0xEC2)
  - `0x00460050` — `BuildingTypeClass::ReadINI_Water` (CanC4 at +0x1577)
  - `0x0066bbd1` — `RulesClass::ReadCombatDamage` (C4Delay at +0x1750, C4Warhead at +0xFA8)
- **INI keys consumed:**
  - `rulesmd.ini [GHOST/TANY/PTROOP] C4=yes`
  - `rulesmd.ini [GAPILE/...] CanC4=no` (defaults yes)
  - `rulesmd.ini [...] InvisibleInGame=yes` (defaults no)
  - `rulesmd.ini [CombatDamage] C4Delay=0.03`
  - `rulesmd.ini [CombatDamage] C4Warhead=Super` (already parsed; no new work)
  - `artmd.ini [SealSequence] FireUp=164,6,6` (drives Attack sequence; no new work)
  - `soundmd.ini [SealPlaceBomb]` (already exists; played from Task 8)
  - `soundmd.ini [SealSpecialAttack]` (already exists; played from Task 12)
- **Related repo code:**
  - [src/sim/command.rs:120-126](../../src/sim/command.rs#L120-L126) — engineer-capture command, pattern source
  - [src/sim/world/world_orders.rs:151-209](../../src/sim/world/world_orders.rs#L151-L209) — tick_capture_orders, pattern source
  - [src/sim/world/world_commands.rs:861-915](../../src/sim/world/world_commands.rs#L861-L915) — capture dispatch, pattern source
  - [src/sim/animation.rs:204](../../src/sim/animation.rs#L204) — Animation::switch_to (drives FireUp/Attack)
  - [src/sim/combat/combat_aoe.rs:33](../../src/sim/combat/combat_aoe.rs#L33) — apply_aoe_damage (NOT used by C4 — CellSpread=0 blocks)
  - [src/app_types.rs:88](../../src/app_types.rs#L88) — CursorId::Demolish (already declared)
  - [src/render/cursor_atlas.rs:202-208](../../src/render/cursor_atlas.rs#L202-L208) — Demolish frames already loaded
