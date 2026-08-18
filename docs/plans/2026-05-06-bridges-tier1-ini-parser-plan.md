# Bridges Tier 1 — INI Parser Fixes Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Close the parser-side parity gaps for the bridge system: fix
the 6× `BridgeStrength` fragility bug, fix the silent-ignore bug on
`DestroyableBridges`, and parse the four bridge INI keys we currently
drop. Pure parser work — no consumers wired this tier.

**Architecture:** Field additions and value updates on `BridgeRules`
([src/rules/ruleset.rs:649](../../src/rules/ruleset.rs#L649)) plus one
new bool flag on `ObjectType`
([src/rules/object_type.rs:855](../../src/rules/object_type.rs#L855)),
following the established `BridgeRules` container pattern and the ~30
existing `ObjectType` per-building bool flags. Sole external consumer
fix: the `unwrap_or(250)` fallback at
[src/app_init_helpers.rs:353](../../src/app_init_helpers.rs#L353).

**Design Doc:** [docs/plans/2026-05-06-bridges-tier1-ini-parser-design.md](2026-05-06-bridges-tier1-ini-parser-design.md)

---

## Grounding Summary

- **Docs:** Direct sources are
  [docs/gap-scans/2026-05-06-gap-scan-bridges-deep.md](../gap-scans/2026-05-06-gap-scan-bridges-deep.md)
  D3 (Constant Drift) and the design doc. Underlying RE evidence sits
  in `ra2-rust-game-docs/BRIDGE_SYSTEM.md`,
  `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`, and
  `CELLCLASS_ZONES_SPEED_BRIDGES.md`.
- **Ghidra verification (already performed in deep audit, this session):**
  `BridgeStrength @ Rules+0x1740` (default 1500), `BridgeVoxelMax @
  Rules+0x624` (default 3), `RepairBridgeSound` string `@ 0x83A7FC`,
  `BridgeRepairHut @ BuildingTypeClass+0x16B6` byte, `DestroyableBridges`
  string `@ 0x840248` only referenced by `FUN_006B8B30` (MP-dialog
  decoder, not the rules.ini reader). **No further Ghidra calls needed
  for this tier.**
- **Repo pattern this mirrors:** `BridgeRules` already has 3 fields and
  a `from_ini` constructor; new fields slot in. `ObjectType` has ~30
  per-building bool flags (`engineer`, `capturable`, `can_be_occupied`,
  `can_occupy_fire`, `show_occupant_pips`, ...) at
  [object_type.rs:855-862](../../src/rules/object_type.rs#L855-L862);
  `bridge_repair_hut` is a peer.
- **INI keys:** `[General] BridgeVoxelMax=3` (rulesmd.ini:419),
  `[AudioVisual] RepairBridgeSound=BridgeRepaired` (rulesmd.ini:721),
  `[CombatDamage] DestroyableBridges=yes` (rulesmd.ini:804),
  `[CombatDamage] BridgeStrength=1500` (rulesmd.ini:816),
  `[CABHUT] BridgeRepairHut=yes` (rulesmd.ini:16348).
- **Still unknown after grounding:** the contested semantics of
  `[CombatDamage] CollapseChance` (INI comment "cliff collapse" vs
  doc claim "bridge state advance" in
  `CELLCLASS_ZONES_SPEED_BRIDGES.md §4.3`). **Deferred to
  `/verify-doc`.** Not in any task in this plan.

## Key Technical Decisions

- **`voxel_max` typed as `u8` with `.clamp(0, 255)`:** matches the
  binary's small integer range and prevents nonsense large values.
  **Confidence:** high. **Source:** `[GHIDRA Rules+0x624]`, `[ini
  rulesmd.ini:419 default=3]`. Even mods rarely exceed single digits.
- **`repair_sound` typed as `Option<String>` rather than `SoundId`:**
  no `[AudioVisual]` named-key parser table exists yet (~80 sound IDs
  are unparsed per the wide gap-scan); `None` lets a Tier 4 consumer
  apply its own default. **Confidence:** high. **Source:** repo state
  inspection — `sound_ini.rs` only has on-demand lookup, no pre-resolved
  named-key table.
- **`bridge_repair_hut` lives on `ObjectType` directly (not on a
  separate `BuildingType` substruct):** matches the codebase's existing
  unified-`ObjectType` pattern. **Confidence:** high. **Source:**
  [object_type.rs:855-862](../../src/rules/object_type.rs#L855-L862).
- **Defer `CollapseChance`:** doc-trust rule on contradicting evidence.
  **Confidence:** high (on the deferral itself). **Source:**
  `[ini rulesmd.ini:908 comment]` vs `[doc CELLCLASS_ZONES_SPEED_BRIDGES.md §4.3]`.

## Open Questions

### Resolved During Planning

- **Will `test_load_ruleset` (the comprehensive smoke test at
  [ruleset.rs:1910](../../src/rules/ruleset.rs#L1910)) need an
  assertion for the new `voxel_max` and `repair_sound` fields?**
  No — its fixture doesn't set those keys; they'll fall back to
  defaults. Coverage of the new fields lives in dedicated tests
  (Tasks 3, 4) to keep `test_load_ruleset` focused on smoke-test
  coverage.
- **Should new tests live in `ruleset.rs` or a separate file?** Keep
  in `ruleset.rs` — that's where existing `bridge_rules_*` tests live
  (e.g., `bridge_rules_load_from_ini` at line 2138). Single-file
  cohesion.

### Deferred to Implementation

None — every detail this tier is fully specified.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/rules/ruleset.rs` (struct, default, from_ini, 2 existing tests + 2 new tests) | `BridgeRules` field set + parser reads |
| Modify | `src/rules/object_type.rs` (struct, parser, 1 new test) | `bridge_repair_hut` field + INI parse |
| Modify | `src/app_init_helpers.rs:353` | One-line constant fix on the `unwrap_or` fallback |

## Interface Changes

- **`BridgeRules`** ([ruleset.rs:651](../../src/rules/ruleset.rs#L651))
  gains two public fields: `pub voxel_max: u8`, `pub repair_sound:
  Option<String>`. No method signatures change. Public, but consumers
  this tier are zero — Tier 2 (damage state machine) and Tier 4
  (repair) will read them later.
- **`ObjectType`** ([object_type.rs:133](../../src/rules/object_type.rs#L133))
  gains one public field: `pub bridge_repair_hut: bool`. Default
  `false`. No consumers this tier.

No interface breaks. Adding fields with defaults; existing consumers
unaffected.

## Sim Checklist

This plan does **not** touch `src/sim/`. All work is in `src/rules/`
and `src/app_init_helpers.rs` (config wiring). Sim invariants are
preserved by construction:

- [x] No `f32`/`f64` introduced (no math).
- [x] No new sim state — new rules fields are config, not state-hashed.
- [x] No `render`/`ui`/`audio`/`net` imports anywhere.
- [x] No tick-ordering impact — runs at app init.
- [x] No `BTreeMap` iteration considerations.

## Risk Areas

- **Existing test `test_load_ruleset` at
  [ruleset.rs:1910](../../src/rules/ruleset.rs#L1910)** asserts
  `strength == 250` at line 1924. Changing the default to 1500
  without updating the assertion in the same task **breaks the
  build**. Task 1 covers both atomically.
- **Existing test `bridge_rules_load_from_ini` at
  [ruleset.rs:2138](../../src/rules/ruleset.rs#L2138)** uses
  `[SpecialFlags] DestroyableBridges=no` in its fixture. Moving the
  parser read without updating the fixture in the same task makes
  the negative assertion fail. Task 2 covers both atomically.
- **Some other test fixture across the codebase silently depends on
  the 250 default.** Mitigated by `cargo test` after Task 1.
- **`get_bool` parsing of `BridgeRepairHut=yes`:** the existing
  pattern at [object_type.rs:855-862](../../src/rules/object_type.rs#L855-L862)
  is `section.get_bool("Foo").unwrap_or(false)`. No risk if mirrored
  exactly.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | `BridgeStrength` default `250 → 1500` | Bridges currently 6× more fragile than gamemd. Fires every wall-warhead bridge hit. | `[ini rulesmd.ini:816]` + `[GHIDRA Rules+0x1740]`; updated assertion in `test_load_ruleset` |
| Task 2 | `DestroyableBridges` section move | Modders/maps that disable bridges via canonical `[CombatDamage]` are silently ignored; only `[SpecialFlags]` (which is wrong) currently works. Fires for any mod or map that sets it. | `[ini rulesmd.ini:804]` + `[GHIDRA FUN_006B8B30 is MP-dialog only]`; new regression test `bridge_rules_destroyable_in_specialflags_is_ignored` |

Tasks 3, 4, 5 are infrastructure (parsed-but-not-yet-consumed) — no
behavioral parity yet, but Tier 2/4 consumers depend on them being
plumbed correctly.

---

## Tasks

### Task 1: Fix `BridgeStrength` default `250 → 1500`

**Why:** Closes the headline 6× fragility bug. Touches three
sites that all hardcode `250`; all must move together to keep the
build green.

**Files:**
- Modify: [src/rules/ruleset.rs:664](../../src/rules/ruleset.rs#L664) (`BridgeRules::default`)
- Modify: [src/rules/ruleset.rs:676](../../src/rules/ruleset.rs#L676) (`BridgeRules::from_ini` fallback)
- Modify: [src/rules/ruleset.rs:1924](../../src/rules/ruleset.rs#L1924) (`test_load_ruleset` assertion)
- Modify: [src/app_init_helpers.rs:353](../../src/app_init_helpers.rs#L353) (consumer fallback)

**Pattern:** existing `BridgeRules` `from_ini` reader pattern;
no structural change.

**Step 1: Update default in `BridgeRules::default`**

In [src/rules/ruleset.rs](../../src/rules/ruleset.rs), inside `impl
Default for BridgeRules`:

```rust
impl Default for BridgeRules {
    fn default() -> Self {
        Self {
            strength: 1500,
            destroyable_by_default: true,
            explosions: Vec::new(),
        }
    }
}
```

**Step 2: Update fallback in `BridgeRules::from_ini`**

In the same file, inside `BridgeRules::from_ini`:

```rust
let strength = ini
    .section("CombatDamage")
    .and_then(|section| section.get_i32("BridgeStrength"))
    .unwrap_or(1500)
    .max(1) as u16;
```

The `.max(1)` clamp is preserved (ledger item #2).

**Step 3: Update consumer fallback**

In [src/app_init_helpers.rs:353](../../src/app_init_helpers.rs#L353):

```rust
let bridge_strength = rules
    .map(|rules| rules.bridge_rules.strength)
    .unwrap_or(1500);
```

**Step 4: Update existing test assertion**

In [src/rules/ruleset.rs:1924](../../src/rules/ruleset.rs#L1924)
(inside `test_load_ruleset`):

```rust
assert_eq!(rules.bridge_rules.strength, 1500);
```

**Step 5: Verify**

Run: `cargo test -p ra2-rust-game --lib bridge_rules`
Run: `cargo test -p ra2-rust-game --lib test_load_ruleset`
Run: `cargo build -p ra2-rust-game`
Expected: all PASS, build clean.

**Step 6: Commit**

```
rules: BridgeStrength default 250 → 1500 (gamemd parity)

The original engine defaults BridgeStrength to 1500 in [CombatDamage]
(rulesmd.ini:816, Rules+0x1740). Our 250 default made bridges 6× more
fragile than gamemd.exe — every wall-warhead hit on a bridge cell did
six times the intended fraction of HP.
```

---

### Task 2: Move `DestroyableBridges` reader to `[CombatDamage]`

**Why:** Closes the silent-ignore bug — any mod or map that disables
bridges via the canonical `[CombatDamage]` section is currently
dropped (we read from `[SpecialFlags]`, which only the MP-dialog
decoder uses in the binary). Includes a regression test pinning the
fix.

**Files:**
- Modify: [src/rules/ruleset.rs:678-680](../../src/rules/ruleset.rs#L678-L680) (`BridgeRules::from_ini` reader)
- Modify: [src/rules/ruleset.rs:2146-2147](../../src/rules/ruleset.rs#L2146-L2147) (test fixture)
- Add to: `src/rules/ruleset.rs` (new test `bridge_rules_destroyable_in_specialflags_is_ignored`)

**Pattern:** existing `from_ini` `ini.section("X").and_then(...)` pattern.

**Step 1: Move the reader**

In [src/rules/ruleset.rs:678-681](../../src/rules/ruleset.rs#L678-L681)
(inside `BridgeRules::from_ini`):

```rust
let destroyable_by_default = ini
    .section("CombatDamage")
    .and_then(|section| section.get_bool("DestroyableBridges"))
    .unwrap_or(true);
```

(Only change: `"SpecialFlags"` → `"CombatDamage"`.)

**Step 2: Update existing test fixture**

In [src/rules/ruleset.rs:2138-2152](../../src/rules/ruleset.rs#L2138-L2152)
(`bridge_rules_load_from_ini`), move `DestroyableBridges=no` from
the `[SpecialFlags]` block to the `[CombatDamage]` block:

```rust
let ini = IniFile::from_str(
    "[InfantryTypes]\n\
     [VehicleTypes]\n\
     [AircraftTypes]\n\
     [BuildingTypes]\n\
     [CombatDamage]\n\
     BridgeStrength=900\n\
     DestroyableBridges=no\n",
);
```

(The `[SpecialFlags]` section is no longer needed for this test.)

**Step 3: Add regression test**

Add a new test next to `bridge_rules_load_from_ini` (in the same
`#[cfg(test)] mod tests` block):

```rust
#[test]
fn bridge_rules_destroyable_in_specialflags_is_ignored() {
    // Regression: gamemd reads DestroyableBridges from [CombatDamage].
    // The string @ 0x840248 in [SpecialFlags] is for MP-dialog overrides,
    // not the rules.ini parser. Putting it under [SpecialFlags] should
    // be silently ignored and the default (yes) kept.
    let ini = IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         [SpecialFlags]\n\
         DestroyableBridges=no\n",
    );
    let rules = RuleSet::from_ini(&ini).expect("Should parse");
    assert!(rules.bridge_rules.destroyable_by_default);
}
```

**Step 4: Verify**

Run: `cargo test -p ra2-rust-game --lib bridge_rules`
Expected: `bridge_rules_load_from_ini` PASS,
`bridge_rules_destroyable_in_specialflags_is_ignored` PASS.

**Step 5: Commit**

```
rules: read DestroyableBridges from [CombatDamage], not [SpecialFlags]

gamemd's only rules.ini reader for DestroyableBridges is in
[CombatDamage] (rulesmd.ini:804). The [SpecialFlags] string @ 0x840248
is referenced only by the multiplayer-dialog scenario-override decoder
(FUN_006B8B30), not the rules parser. Mods/maps disabling bridges via
the canonical section were silently ignored. Adds a regression test
pinning the fix.
```

---

### Task 3: Add `voxel_max` field

**Why:** Plumbs `[General] BridgeVoxelMax` for the Tier 2 damage state
machine's debris loop (binary uses `Rules+0x624` to cap MetallicDebris
spawns per cell). Parsed-but-unread this tier; consumer lands later.

**Files:**
- Modify: [src/rules/ruleset.rs](../../src/rules/ruleset.rs)
  (`BridgeRules` struct, default, `from_ini`, 2 tests)

**Pattern:** existing `BridgeRules` field pattern; integer `from_ini`
read with `unwrap_or(default).clamp(...)` pattern (mirrors `strength`'s
`.max(1)` clamp).

**Step 1: Add field to struct**

In [src/rules/ruleset.rs:651](../../src/rules/ruleset.rs#L651):

```rust
#[derive(Debug, Clone)]
pub struct BridgeRules {
    /// Hit points shared by a destroyable bridge span.
    pub strength: u16,
    /// Whether bridges are destroyable unless the map overrides it.
    pub destroyable_by_default: bool,
    /// SHP animation names to spawn when a bridge group is destroyed
    /// (e.g., TWLT026, TWLT036, TWLT050, TWLT070). Picked randomly per cell.
    pub explosions: Vec<String>,
    /// Maximum metallic-debris voxels spawned per destroyed bridge cell.
    /// Parsed from `[General] BridgeVoxelMax=` in rules.ini (default 3).
    /// Consumed by the damage state machine in a later tier.
    pub voxel_max: u8,
}
```

**Step 2: Update `Default`**

In `impl Default for BridgeRules`:

```rust
impl Default for BridgeRules {
    fn default() -> Self {
        Self {
            strength: 1500,
            destroyable_by_default: true,
            explosions: Vec::new(),
            voxel_max: 3,
        }
    }
}
```

**Step 3: Add `from_ini` read**

Inside `BridgeRules::from_ini`, after the `explosions` parse:

```rust
let voxel_max = ini
    .section("General")
    .and_then(|section| section.get_i32("BridgeVoxelMax"))
    .unwrap_or(3)
    .clamp(0, 255) as u8;
```

And include it in the returned struct literal:

```rust
Self {
    strength,
    destroyable_by_default,
    explosions,
    voxel_max,
}
```

**Step 4: Extend `bridge_rules_load_from_ini` fixture**

Update the test fixture and assertion to cover the new field:

```rust
#[test]
fn bridge_rules_load_from_ini() {
    let ini = IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         [General]\n\
         BridgeVoxelMax=5\n\
         [CombatDamage]\n\
         BridgeStrength=900\n\
         DestroyableBridges=no\n",
    );
    let rules = RuleSet::from_ini(&ini).expect("Should parse");
    assert_eq!(rules.bridge_rules.strength, 900);
    assert!(!rules.bridge_rules.destroyable_by_default);
    assert_eq!(rules.bridge_rules.voxel_max, 5);
}
```

**Step 5: Add clamp regression test**

Add after `bridge_rules_destroyable_in_specialflags_is_ignored`:

```rust
#[test]
fn bridge_rules_voxel_max_clamps_oversize() {
    // Regression: u8 storage clamps oversize INI values to 255 instead
    // of wrapping/truncating.
    let ini = IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         [General]\n\
         BridgeVoxelMax=999\n",
    );
    let rules = RuleSet::from_ini(&ini).expect("Should parse");
    assert_eq!(rules.bridge_rules.voxel_max, 255);
}
```

**Step 6: Verify**

Run: `cargo test -p ra2-rust-game --lib bridge_rules`
Run: `cargo build -p ra2-rust-game`
Expected: all PASS, build clean. The existing `test_load_ruleset` is
unaffected (its fixture doesn't set `BridgeVoxelMax`, so the default
`3` is used silently — no assertion needs adding there).

**Step 7: Commit**

```
rules: parse [General] BridgeVoxelMax (default 3, clamp 0..=255)

Plumbs the per-cell metallic-debris cap (Rules+0x624 in gamemd) used by
the bridge damage state machine. Parsed-but-unread this tier; consumer
lands when the state machine ships.
```

---

### Task 4: Add `repair_sound` field

**Why:** Plumbs `[AudioVisual] RepairBridgeSound` for the Tier 4 bridge
repair handler (Engineer entry into BridgeRepairHut → fire `EVA_BridgeRepaired`
+ `RepairBridgeSound`). Stored as `Option<String>`; resolution to a
sound entry happens at the call site when the consumer lands. Avoids
introducing an `[AudioVisual]` named-key parser table for a single key.

**Files:**
- Modify: [src/rules/ruleset.rs](../../src/rules/ruleset.rs)
  (`BridgeRules` struct, default, `from_ini`, extend
  `bridge_rules_load_from_ini`)

**Pattern:** uppercased optional-string read, mirrors
[ruleset.rs:706-711](../../src/rules/ruleset.rs#L706-L711) (the
`parse_anim_name` helper) and the `parachute_shp` parse at
[ruleset.rs:733-736](../../src/rules/ruleset.rs#L733-L736).

**Step 1: Add field to struct**

In [src/rules/ruleset.rs:651](../../src/rules/ruleset.rs#L651), append
to `BridgeRules`:

```rust
    /// Sound ID played when a bridge segment is repaired by an
    /// Engineer entering a `BridgeRepairHut=yes` building.
    /// Parsed from `[AudioVisual] RepairBridgeSound=` in rules.ini
    /// (stock default `BridgeRepaired`). Stored uppercased.
    /// `None` means the consumer applies its own default.
    pub repair_sound: Option<String>,
```

**Step 2: Update `Default`**

```rust
impl Default for BridgeRules {
    fn default() -> Self {
        Self {
            strength: 1500,
            destroyable_by_default: true,
            explosions: Vec::new(),
            voxel_max: 3,
            repair_sound: None,
        }
    }
}
```

**Step 3: Add `from_ini` read**

Inside `BridgeRules::from_ini`, after the `voxel_max` parse:

```rust
let repair_sound = ini
    .section("AudioVisual")
    .and_then(|section| section.get("RepairBridgeSound"))
    .map(|s| s.trim().to_uppercase())
    .filter(|s| !s.is_empty());
```

And include it in the returned struct literal:

```rust
Self {
    strength,
    destroyable_by_default,
    explosions,
    voxel_max,
    repair_sound,
}
```

**Step 4: Extend `bridge_rules_load_from_ini` fixture**

```rust
#[test]
fn bridge_rules_load_from_ini() {
    let ini = IniFile::from_str(
        "[InfantryTypes]\n\
         [VehicleTypes]\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         [General]\n\
         BridgeVoxelMax=5\n\
         [AudioVisual]\n\
         RepairBridgeSound=foo\n\
         [CombatDamage]\n\
         BridgeStrength=900\n\
         DestroyableBridges=no\n",
    );
    let rules = RuleSet::from_ini(&ini).expect("Should parse");
    assert_eq!(rules.bridge_rules.strength, 900);
    assert!(!rules.bridge_rules.destroyable_by_default);
    assert_eq!(rules.bridge_rules.voxel_max, 5);
    assert_eq!(rules.bridge_rules.repair_sound.as_deref(), Some("FOO"));
}
```

**Step 5: Verify**

Run: `cargo test -p ra2-rust-game --lib bridge_rules`
Run: `cargo build -p ra2-rust-game`
Expected: all PASS, build clean. Default-case (no `RepairBridgeSound=`
in fixture) gives `None`, asserted indirectly by `test_load_ruleset`
remaining green.

**Step 6: Commit**

```
rules: parse [AudioVisual] RepairBridgeSound (Option<String>, uppercased)

Plumbs the bridge-repaired sound ID for the Tier 4 repair handler.
Resolution to a SoundEntry deferred to the consumer call site to avoid
prematurely introducing an [AudioVisual] named-key parser table.
```

---

### Task 5: Add `bridge_repair_hut: bool` to `ObjectType`

**Why:** Plumbs the per-building `BridgeRepairHut=yes` flag (CABHUT in
stock) so the Tier 4 repair handler can detect the trigger building.
Parsed-but-unread this tier.

**Files:**
- Modify: [src/rules/object_type.rs](../../src/rules/object_type.rs)
  (`ObjectType` struct field, `from_ini_section` parser, new test)

**Pattern:** existing per-building bool flags at
[object_type.rs:855-862](../../src/rules/object_type.rs#L855-L862):
field declared on the struct with a doc comment, parsed via
`section.get_bool("Foo").unwrap_or(false)`.

**Step 1: Add field to struct**

In [src/rules/object_type.rs](../../src/rules/object_type.rs), add a
field next to the other per-building flags (immediately after
`show_occupant_pips` at line 445 is a natural spot):

```rust
    /// Whether Engineer entry into this building triggers bridge-segment
    /// repair on the nearest damaged bridge. Parsed from
    /// `BridgeRepairHut=yes` in rules.ini. Stock CABHUT is the only
    /// consumer in retail. Default `false`.
    pub bridge_repair_hut: bool,
```

**Step 2: Add parser line**

In `ObjectType::from_ini_section`, in the same block as the existing
per-building bool flags around
[object_type.rs:855-862](../../src/rules/object_type.rs#L855-L862),
add (matching the surrounding indentation):

```rust
            bridge_repair_hut: section.get_bool("BridgeRepairHut").unwrap_or(false),
```

(Place it adjacent to `repairable` / `can_be_occupied` / `can_occupy_fire`
/ `show_occupant_pips` for cohesion.)

**Step 3: Add test**

In the `#[cfg(test)] mod tests` block at the bottom of
`object_type.rs`, add the following. This mirrors
`parse_no_force_shield_flag` at
[object_type.rs:1100-1115](../../src/rules/object_type.rs#L1100):

```rust
#[test]
fn parse_bridge_repair_hut_flag() {
    let ini: IniFile = IniFile::from_str("[CABHUT]\nBridgeRepairHut=yes\n[NACABH]\n");
    let obj_on: ObjectType = ObjectType::from_ini_section(
        "CABHUT",
        ini.section("CABHUT").unwrap(),
        ObjectCategory::Building,
    );
    let obj_off: ObjectType = ObjectType::from_ini_section(
        "NACABH",
        ini.section("NACABH").unwrap(),
        ObjectCategory::Building,
    );
    assert!(obj_on.bridge_repair_hut);
    assert!(!obj_off.bridge_repair_hut);
}
```

Signature reference: `ObjectType::from_ini_section(id: &str, section: &IniSection, category: ObjectCategory) -> Self` at [object_type.rs:646](../../src/rules/object_type.rs#L646). Returns `Self` (not `Result`); `IniFile::section` returns `Option<&IniSection>`.

**Step 4: Verify**

Run: `cargo test -p ra2-rust-game --lib bridge_repair_hut`
Run: `cargo build -p ra2-rust-game`
Expected: PASS, build clean.

**Step 5: Commit**

```
rules: parse BridgeRepairHut=yes onto ObjectType (default false)

Stock CABHUT sets this flag; gamemd uses BuildingTypeClass+0x16B6 to
gate Engineer-entry → 5×5 bridge-segment repair scan. Parsed-but-unread
this tier; Tier 4 repair handler consumes it.
```

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-06-bridges-tier1-ini-parser-design.md](2026-05-06-bridges-tier1-ini-parser-design.md)
- **Deep gap-scan:** [docs/gap-scans/2026-05-06-gap-scan-bridges-deep.md](../gap-scans/2026-05-06-gap-scan-bridges-deep.md)
- **Ghidra reports:**
  `ra2-rust-game-docs/BRIDGE_SYSTEM.md`,
  `ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`,
  `ra2-rust-game-docs/CELLCLASS_ZONES_SPEED_BRIDGES.md`,
  `ra2-rust-game-docs/LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md`,
  `ra2-rust-game-docs/BRIDGE_RENDERING_GHIDRA_REPORT.md`.
- **gamemd.exe addresses (verified live in deep audit, this session):**
  `BridgeStrength` Rules+0x1740 (default 1500),
  `BridgeVoxelMax` Rules+0x624 (default 3),
  `BridgeRepairHut` BuildingTypeClass+0x16B6,
  `RepairBridgeSound` string @ 0x83A7FC,
  `DestroyableBridges` string @ 0x840248 (referenced only by
  `FUN_006B8B30` MP-dialog decoder).
- **INI keys:**
  `[General] BridgeVoxelMax=3` (rulesmd.ini:419),
  `[AudioVisual] RepairBridgeSound=BridgeRepaired` (rulesmd.ini:721),
  `[CombatDamage] DestroyableBridges=yes` (rulesmd.ini:804),
  `[CombatDamage] BridgeStrength=1500` (rulesmd.ini:816),
  `[CombatDamage] CollapseChance=100` (rulesmd.ini:908) **[deferred —
  /verify-doc CELLCLASS_ZONES_SPEED_BRIDGES.md before parsing]**,
  `[CABHUT] BridgeRepairHut=yes` (rulesmd.ini:16348).
- **Related code (existing patterns mirrored):**
  `BridgeRules` at [src/rules/ruleset.rs:649-693](../../src/rules/ruleset.rs#L649-L693),
  per-building bool flag pattern at
  [src/rules/object_type.rs:855-862](../../src/rules/object_type.rs#L855-L862),
  consumer fallback at
  [src/app_init_helpers.rs:343-369](../../src/app_init_helpers.rs#L343-L369).
