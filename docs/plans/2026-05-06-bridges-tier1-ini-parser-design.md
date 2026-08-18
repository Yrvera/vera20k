# Bridges Tier 1 — INI Parser Fixes Design

## Goal

Close the parser-side parity gaps for the bridge system identified by
`docs/gap-scans/2026-05-06-gap-scan-bridges-deep.md` Tier 1: fix the 6×
`BridgeStrength` fragility bug, fix the silent-ignore bug on
`DestroyableBridges`, and parse the four bridge INI keys we currently
drop. All work is parser-side; consumers of the new fields land in
later tiers.

## Architecture Context

- `BridgeRules` lives in [src/rules/ruleset.rs:649-693](../../src/rules/ruleset.rs#L649-L693)
  as the global bridge-rules container (sibling to `GeneralRules`,
  `CombatDamageDefaults`, etc.). Currently holds `strength`,
  `destroyable_by_default`, `explosions`. Reachable through
  `RuleSet.bridge_rules` ([ruleset.rs:1103](../../src/rules/ruleset.rs#L1103)),
  populated via `BridgeRules::from_ini` ([ruleset.rs:1138](../../src/rules/ruleset.rs#L1138)).
- Two consumers, both at sim init time:
  [app_init_helpers.rs:343-369](../../src/app_init_helpers.rs#L343-L369)
  reads `destroyable_by_default` (combined with the per-map
  `SpecialFlags.destroyable_bridges` override from
  [src/map/basic.rs:36](../../src/map/basic.rs#L36)) and `strength`
  to seed `BridgeRuntimeState`. The `strength` reader has its own
  fallback `unwrap_or(250)` at line 353.
- `ObjectType` in [src/rules/object_type.rs](../../src/rules/object_type.rs)
  is the unified type for all object kinds (`InfantryType`,
  `VehicleType`, `BuildingType`, etc.) discriminated by a `kind` field.
  Building-specific bool flags (e.g., `can_be_occupied`,
  `can_occupy_fire`, `show_occupant_pips`) live directly on the struct
  with the convention `pub <name>: bool,` + doc comment "Parsed from
  `XXX=yes` in rules.ini." `bridge_repair_hut` slots in identically.
- No existing `[AudioVisual]` named-sound-ID parser pattern exists.
  `RepairBridgeSound` is one of ~80 such keys flagged in
  `docs/gap-scans/2026-05-06-gap-scan.md`. This design stores the raw
  string on `BridgeRules` and defers any sound-table infrastructure to
  the broader `[AudioVisual]` parser thread.
- INI section locations confirmed in `ini/rulesmd.ini`:
  `BridgeVoxelMax` line 419 (`[General]`), `RepairBridgeSound` line
  721 (`[AudioVisual]`), `DestroyableBridges` line 804 (`[CombatDamage]`),
  `BridgeStrength` line 816 (`[CombatDamage]`), `BridgeRepairHut` line
  16348 (on `[CABHUT]`).

## Impact Analysis

**Files touched:**

| File | Change |
|---|---|
| [src/rules/ruleset.rs](../../src/rules/ruleset.rs) | `BridgeRules` struct: add `voxel_max: u8` and `repair_sound: Option<String>` fields. `Default::default()`: `strength: 1500`, `voxel_max: 3`. `from_ini`: change `[SpecialFlags]` → `[CombatDamage]` for `DestroyableBridges`, change fallback `unwrap_or(250)` → `unwrap_or(1500)`, parse `[General] BridgeVoxelMax`, parse `[AudioVisual] RepairBridgeSound`. Update `bridge_rules_default_*` and `bridge_rules_load_from_ini` tests. |
| [src/rules/object_type.rs](../../src/rules/object_type.rs) | Add `pub bridge_repair_hut: bool,` field with doc comment, parse `BridgeRepairHut=yes` in `from_ini_section` (default `false`). |
| [src/app_init_helpers.rs:353](../../src/app_init_helpers.rs#L353) | Change `.unwrap_or(250)` → `.unwrap_or(1500)`. |

**Dependencies in the graph:**
- `RuleSet` is loaded once at app init; new fields propagate freely.
- New `BridgeRules` fields have **no consumers yet** — Tier 2 (damage
  state machine) will read `voxel_max`, Tier 4 (repair) will read
  `repair_sound`, and the in-progress occupancy/repair work will read
  `bridge_repair_hut`.
- Determinism: state hash is unaffected because no sim state changes.
- Save/load: snapshot serialization is a separate workstream
  (`project_snapshot_serialization.md`); these fields live on rules
  (config), not sim state, so they're outside that scope.

**What might break:**
- The existing test `bridge_rules_load_from_ini` ([ruleset.rs:2138-2152](../../src/rules/ruleset.rs#L2138-L2152))
  has `[SpecialFlags] DestroyableBridges=no` in its fixture. After the
  parser change, that fixture must move the line to `[CombatDamage]`,
  otherwise the assertion `!destroyable_by_default` fails.
- The existing test `bridge_rules_defaults` (or whichever sets the
  baseline at [ruleset.rs:1924](../../src/rules/ruleset.rs#L1924))
  asserts `strength == 250`. Update to `1500`.
- Risk that some other test fixture across the codebase silently
  depends on the 250 default. Mitigated by `cargo test` after change.

**Blast radius:** rules-parser only. No sim, render, audio, network,
or save/load surface changes.

## Chosen Approach

**Approach A** from brainstorm: single commit, parser-only, follows
established patterns end-to-end.

- `BridgeRules` grows by 2 fields. Continues the existing pattern of
  a single global rules container per system.
- `ObjectType.bridge_repair_hut` is a peer to ~30 existing per-building
  bool flags. No new pattern.
- New fields are intentionally parsed-but-unread. Tier 2/4 work picks
  them up later without re-touching the parser.
- `CollapseChance` (deferred): doc-trust red flag — the INI comment
  ("cliff collapse") and `CELLCLASS_ZONES_SPEED_BRIDGES.md §4.3`
  ("bridge state advance") contradict. **Out of Tier 1 scope. Run
  `/verify-doc CELLCLASS_ZONES_SPEED_BRIDGES.md` before parsing.**

## Tiny-Detail Ledger

The implementation must preserve all of these. Each item has a designated
home in this design.

| # | Detail | Source | Home in design |
|---|--------|--------|----------------|
| 1 | `BridgeStrength` default = **1500** | `[ini: rulesmd.ini:816]`, `[GHIDRA Rules+0x1740]` | `BridgeRules::default().strength = 1500` + `from_ini` `unwrap_or(1500)` + `app_init_helpers.rs:353` `unwrap_or(1500)` |
| 2 | `BridgeStrength` clamped to `>= 1` | current code | `.max(1)` preserved in `from_ini` |
| 3 | `DestroyableBridges` lives in `[CombatDamage]` | `[ini: rulesmd.ini:804]`, `[GHIDRA FUN_006B8B30 is MP-dialog only — not the rules.ini reader]` | `from_ini` reads `ini.section("CombatDamage")` |
| 4 | `DestroyableBridges` default = `yes` | `[ini: rulesmd.ini:804]` | `unwrap_or(true)` |
| 5 | `BridgeVoxelMax` lives in `[General]`, default = **3**, integer count | `[ini: rulesmd.ini:419]`, `[GHIDRA Rules+0x624]` | `BridgeRules::default().voxel_max = 3` + `from_ini` reads `[General]` with `unwrap_or(3).clamp(0, 255) as u8` |
| 6 | `RepairBridgeSound` lives in `[AudioVisual]`, default value `BridgeRepaired` | `[ini: rulesmd.ini:721]`, `[GHIDRA string @ 0x83A7FC]` | `from_ini` reads `[AudioVisual]` as `Option<String>`. `None` is left as `None` (consumer applies its own default when Tier 4 lands; no early commitment) |
| 7 | Sound IDs uppercased on read (codebase convention) | `sound_ini.rs:166` | `.map(\|s\| s.trim().to_uppercase()).filter(\|s\| !s.is_empty())` |
| 8 | `BridgeRepairHut` is per-building bool, default `false` | `[ini: rulesmd.ini:16348]`, `[GHIDRA BuildingTypeClass+0x16B6]` | `ObjectType.bridge_repair_hut: bool` parsed from `BridgeRepairHut=yes` |
| 9 | `CollapseChance` semantics contested | `[ini comment: cliff collapse]` vs `[doc: CELLCLASS_ZONES_SPEED_BRIDGES.md §4.3 — bridge state advance]` | **Deferred. `/verify-doc` recommended.** TODO note in design doc, no parser change. |
| 10 | Existing `bridge_rules_default_*` test asserts `strength == 250` | [ruleset.rs:1924](../../src/rules/ruleset.rs#L1924) | Update assertion to `1500`. |
| 11 | Existing `bridge_rules_load_from_ini` test fixture uses `[SpecialFlags]` | [ruleset.rs:2146-2147](../../src/rules/ruleset.rs#L2146-L2147) | Move `DestroyableBridges=no` line to `[CombatDamage]` block. |
| 12 | Fallback `unwrap_or(250)` at [app_init_helpers.rs:353](../../src/app_init_helpers.rs#L353) | current code | Update to `1500`. |

## Design

### Components

No new components. All changes are field additions / value updates on
existing structs.

```rust
// src/rules/ruleset.rs

pub struct BridgeRules {
    pub strength: u16,
    pub destroyable_by_default: bool,
    pub explosions: Vec<String>,
    pub voxel_max: u8,                 // NEW
    pub repair_sound: Option<String>,  // NEW
}

impl Default for BridgeRules {
    fn default() -> Self {
        Self {
            strength: 1500,             // CHANGED 250 → 1500
            destroyable_by_default: true,
            explosions: Vec::new(),
            voxel_max: 3,               // NEW
            repair_sound: None,         // NEW
        }
    }
}
```

```rust
// src/rules/object_type.rs

pub struct ObjectType {
    // ... existing ~30 bool fields ...

    /// Whether infantry-Engineer entry triggers bridge repair on the
    /// nearest low/high bridge tile. Parsed from `BridgeRepairHut=yes`
    /// in rules.ini (CABHUT in stock).
    pub bridge_repair_hut: bool,       // NEW
}
```

### Interfaces / Contracts

`BridgeRules::from_ini(ini: &IniFile) -> Self` is unchanged in
signature; internal section reads expanded.

`ObjectType::from_ini_section(...)` is unchanged in signature; one
extra `get_bool("BridgeRepairHut").unwrap_or(false)` line added in the
appropriate field-init block.

### Data Flow

```
ini/rulesmd.ini ──parse──▶ IniFile
                              │
                              ├──▶ BridgeRules::from_ini ──▶ RuleSet.bridge_rules
                              │       (strength, destroyable_by_default,
                              │        explosions, voxel_max, repair_sound)
                              │
                              └──▶ ObjectType::from_ini_section ──▶ ObjectType
                                      (bridge_repair_hut, ...)

RuleSet.bridge_rules ──▶ app_init_helpers.rs:343-369 ──▶ BridgeRuntimeState
                          (strength, destroyable_by_default;
                           voxel_max + repair_sound + bridge_repair_hut
                           UNREAD this tier — Tier 2/4 will plumb)
```

### Error Handling

INI parser uses `Option`-returning getters. All new fields fall back
to documented defaults on missing keys (`thiserror`/`anyhow` not
needed). No I/O. No new failure modes.

### Testing Strategy

Modify two existing tests, add three new tests. All in
`src/rules/ruleset.rs` or `src/rules/object_type.rs`.

1. **`bridge_rules_defaults` (existing, line ~1924).** Update
   `assert_eq!(rules.bridge_rules.strength, 250)` →
   `assert_eq!(rules.bridge_rules.strength, 1500)`. Add
   `assert_eq!(rules.bridge_rules.voxel_max, 3)` and
   `assert!(rules.bridge_rules.repair_sound.is_none())`.

2. **`bridge_rules_load_from_ini` (existing, line 2138).** Move
   `DestroyableBridges=no` from the `[SpecialFlags]` block to the
   existing `[CombatDamage]` block. Extend fixture with
   `BridgeVoxelMax=5` (under `[General]`) and `RepairBridgeSound=foo`
   (under `[AudioVisual]`). Add asserts:
   - `bridge_rules.voxel_max == 5`
   - `bridge_rules.repair_sound.as_deref() == Some("FOO")` (uppercased)

3. **NEW `bridge_rules_destroyable_in_specialflags_is_ignored`.**
   Fixture has `[SpecialFlags] DestroyableBridges=no` ONLY. Assert
   `destroyable_by_default == true` (default). Pins the regression: a
   future revert to the wrong section would fail this test.

4. **NEW `bridge_rules_voxel_max_clamps_oversize`.** Fixture has
   `[General] BridgeVoxelMax=999`. Assert `voxel_max == 255`. Pins the
   `u8` clamp.

5. **NEW `building_type_bridge_repair_hut_parses`.** Fixture is a
   minimal CABHUT-style building section with `BridgeRepairHut=yes`.
   Assert `obj.bridge_repair_hut == true`. Add a sibling minimal
   building section with `BridgeRepairHut=no` (or absent) and assert
   `false`.

`cargo test -p ra2-rust-game` runs everything. No integration test
needed; this is pure parser work.

## Architectural Decisions

- **Follow `BridgeRules` container pattern.** New fields slot in;
  no new module, no new trait. Matches `GeneralRules`,
  `CombatDamageDefaults`, etc.
- **Follow `ObjectType` bool-flag pattern.** No separate
  `BuildingType` struct; the unified `ObjectType` already carries
  building-specific flags.
- **Defer `[AudioVisual]` sound-table infrastructure.** Storing
  `repair_sound` as `Option<String>` keeps Tier 1 narrow. Resolution
  to a `SoundEntry` (via `sound_ini.rs`) happens at Tier 4 call sites
  or whenever the broader `[AudioVisual]` parser lands.
- **Defer `CollapseChance`.** Doc-trust rule: contradicting evidence
  → `/verify-doc` before parsing. Explicit out-of-scope, with a
  pointer to the doc that needs verification.
- **No tech debt introduced.** Three new fields are unread; this is
  expected for a parser-prep tier and matches the wider plan
  (Tier 2/3/4 are explicit follow-ups).

## Alternatives Considered

- **Approach B (split into 2 commits — bug-fix, then field-add).**
  Rejected: per `CLAUDE.md` Git workflow, commits go directly to
  `dev` without PR review, so the split adds churn for no review
  benefit. The bug-fix and test-fixture moves are also tightly
  entangled — splitting them would either keep test changes in
  commit 1 (fine, but then "bug-fix" isn't atomic) or skip them and
  break the build mid-sequence (not fine).
- **Approach C (bundle adjacent cleanups: `BridgeExplosions` Z-offset
  fix in `world/mod.rs:882`, parse a few `[AudioVisual]` sound IDs
  while we're here).** Rejected: scope creep. The Z-offset fix is a
  sim/runtime change not a parser change; the `[AudioVisual]` table
  is its own ~80-key workstream tracked in the wide gap-scan.
- **Resolve `repair_sound` to a `SoundId` at parse time.** Rejected:
  there's no `[AudioVisual]` named-key parser table yet, and building
  one for one key is premature infrastructure.
