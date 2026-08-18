# Entity-State Authority Substrate — Shadow Foundation Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained. Stop
> at the phase boundary; this plan does not authorize a gameplay authority flip.

**Goal:** Add exact entity-state representations, an embedded non-authoritative
shadow bundle, and comparison/access contracts without changing gameplay decisions,
RNG, snapshots, state hashes, lifecycle authority, or reference authority.

**Architecture:** `GameEntity` continues to own legacy live fields and gains a
`#[serde(skip)]` exact shadow bundle. A focused view/mutation facade exposes the
bundle and produces fixed-size diagnostics. Legacy behavior remains authoritative;
the shadow is rebuilt from legacy state at construction and after load. The already
landed ordered lifecycle implementation remains the only owner of reveal, conceal,
limbo, UnInit, pending-delete, and LogicVector membership.

**Design Doc:**
`docs/plans/2026-07-22-entity-state-authority-substrate-design.md`

**Execution worktree:**
`<local>/Documents/ra2-rust-game-gsi-08-10-damage-authority`
on branch `feature/gsi-08-10-damage-authority`.

**Current phase boundary:** Tasks 1–6 are safe shadow-foundation work. Do not route
live damage, veterancy, readiness, modifier, or relation readers/writers through the
new authority yet. Do not change `SNAPSHOT_VERSION`. Do not touch
`src/sim/world/techno_ai.rs`.

**Review correction (2026-07-23):** The pre-execution review's two confirmed
blockers are incorporated: Task 1 now preserves native empty-overlay ability
semantics and updates every direct `ObjectType` fixture. Its whitespace-token test
also closes the review's theoretical parser concern.

---

## Grounding Summary

- The approved design contains Goal, Architecture Context, Impact Analysis,
  component contracts, shadow comparison rules, testing, research gates, and a
  coordinated-cutover gate.
- The research index was valid, but the broad `system=damage` topic filter returned
  no rows. Exact anchors and unfiltered handoffs resolved the current primary docs
  and Rust touchpoints.
- Fresh read-only Ghidra assembly on active `gamemd.exe` reconfirmed that
  `ObjectClass::ReceiveDamage @ 0x005F5390` reads/writes Health and Strength as
  dwords and that `TechnoClass::ReceiveDamage @ 0x00701900` reads readiness/current
  ammo as dwords.
- Fresh assembly also reconfirmed the veterancy comparisons at `0x0074FF90` and
  `0x00750010` against native float constants `1.0f` and `2.0f`.
- The July 13 receiver synthesis classifies the ordered signed transaction as
  verified for its bounded core but leaves G1 **FAILED**. It explicitly permits
  schema-only shadow work and forbids treating the shadow receiver as live authority.
- The exact 18-byte Veteran/Elite array storage, corrected name-to-index table, and
  elite OR-with-veteran behavior are verified. Current Rust keeps only two derived
  FEARLESS booleans.
- Fresh parser checks at `0x00477640` and `0x00528A10` refine “present replaces”:
  only a non-empty `ReadString` result replaces the 18 bytes. An empty value follows
  the fallback path and preserves the prior array. Native tokenization uses comma as
  its only delimiter (`0x00817F70`) and does not trim whitespace around interior
  tokens.
- Current `GameEntity` stores `Health { current: u16, max: u16 }`, `veterancy: u16`,
  `last_attacker_id`, and optional aircraft-only ammo state.
- Current live combat still applies unsigned damage with `saturating_sub`; the pure
  damage service is shadow-only and contains stale behavior relative to the July 13
  synthesis. This plan does not wire it live.
- `NativeF32Bits`, `NativeF64Bits`, and the deterministic x87 subset already exist
  in `src/util/native_x87.rs`; no Rust `f32`/`f64` gameplay storage is needed.
- Commit `95bef99d` has already landed ordered lifecycle authority. The new substrate
  must hand future fatal outcomes to `LifecycleRequest::Uninit`, never infer object
  membership from health.
- The closest repository pattern is the production and bridge shadow discipline:
  non-authoritative, non-serialized, non-hashed state with divergences surfaced and
  a separately reviewed authority flip.
- Stock INI inputs relevant to this phase are `[General] VeteranRatio`,
  `VeteranCombat`, `VeteranArmor`, `VeteranCap`, object-type `VeteranAbilities`,
  `EliteAbilities`, and finite `Ammo`. `DamageReducesReadiness`, `InitialAmmo`, and
  the full reload cluster have no complete current Rust ownership and are excluded.
- Unknowns after grounding remain the reload middle field/consumers, full readiness
  writer lifecycle, per-instance firepower writer lifecycle, House combat-field
  persistence/reapply lifecycle, receiver argument 6, concrete wrapper gaps, and
  reference-authority final shape.

## Key Technical Decisions

- **Store signed vitality as `i32` in the shadow bundle.** Native receiver dword
  reads/writes are reconfirmed; legacy `u16` remains live in this phase.
  — **Confidence: high**
  - **Source:**
    `DAMAGE_RECEIVER_CORE_REINVESTIGATION_2026-07-13.md`; fresh assembly at
    `0x005F5390`.
- **Store experience as `NativeF32Bits` and derive rank through `X87Chop53`.** This
  preserves native state bits without ordinary Rust floating-point gameplay math.
  — **Confidence: high**
  - **Source:** `VETERANCY_SYSTEM_GHIDRA_REPORT.md`; fresh assembly at
    `0x0074FF90` and `0x00750010`; repo pattern `src/util/native_x87.rs`.
- **Represent all 18 Veteran/Elite bytes, while retaining existing FEARLESS fields
  as compatibility projections.** The arrays are type/rules state, not entity state.
  Preserve an earlier array when an overlay supplies an empty value, and parse
  non-empty values with native comma-only tokenization rather than the generic
  whitespace-trimming list helper.
  — **Confidence: high**
  - **Source:** `VETERANCY_SYSTEM_GHIDRA_REPORT.md:258-306` and
    `DAMAGE_RECEIVER_CORE_REINVESTIGATION_2026-07-13.md:205`; fresh live
    decompilation at `0x00477640` and `0x00528A10`, plus delimiter bytes at
    `0x00817F70`.
- **Use `ExperienceShadow::LegacyOnly(u16)` for noncanonical legacy values.** Do not
  fabricate a native float from a centirank value whose writer semantics are not
  proven. Comparisons classify it as `Uncomparable`.
  — **Confidence: medium**
  - **Source:** approved design's unknown-semantics rule plus current Rust storage;
    migration policy, not a gamemd behavior claim.
- **Initialize per-instance armor/firepower shadow defaults to exact native `1.0`
  bits, but expose no live writer or authority.** Constructor defaults are verified;
  complete firepower writers are not.
  — **Confidence: high for defaults; low for later writer coverage**
  - **Source:**
    `DAMAGE_RECEIVER_RULE_HOUSE_ASSEMBLY_REINVESTIGATION_2026-07-13.md:142-150`.
- **Represent readiness only when a current Rust finite-aircraft state exists.** This
  is a lossless shadow projection, not a claim that aircraft docking state equals the
  general native Techno reload scheduler.
  — **Confidence: high**
  - **Source:** current `AircraftAmmo`; failed G1 readiness provenance rows.
- **Keep shadow state skipped by serde and absent from `world_hash`.** Rebuild it from
  legacy fields after load; keep snapshot version 28.
  — **Confidence: high**
  - **Source:** approved design; production/bridge shadow patterns; current snapshot
    rebuild architecture.
- **Include only owner and last-attacker in the initial relation shadow.** Controller
  and detach/final-reference semantics stay excluded until the separate reference
  authority work lands.
  — **Confidence: medium**
  - **Source:** current Rust field ownership and approved design boundary.

The medium/low-confidence migration decisions were checked by `/review-plan`; no
blocker was found in those decisions. They remain intentionally non-authoritative
and cannot justify parity claims.

## Open Questions

### Resolved During Planning

- **Has lifecycle authority landed?** Yes. Commit `95bef99d` added independent
  `ObjectLifecycle`, ordered reveal/conceal/UnInit, `LifecycleRequest`, LogicVector
  membership, and pending-delete handling.
- **Can exact float state be stored without Rust floating-point simulation math?**
  Yes. `NativeF32Bits`, `NativeF64Bits`, and `X87Chop53` already provide the required
  deterministic representation and comparisons.
- **Can the new bundle remain snapshot/hash neutral?** Yes. Use `#[serde(skip)]`, no
  hash fold, rebuild after load, and verify serialized bytes/hash are invariant under
  shadow-only mutation.
- **Where should finite aircraft readiness be sampled?** At the centralized
  `store_spawned_limbo` boundary and after snapshot load, after optional aircraft
  state has been populated.
- **Does `VeteranAbilities=` clear a previously parsed array?** No. The empty
  `ReadString` result is zero-length, so `0x00477640` copies the prior 18 bytes. The
  merged Rust INI path must preserve the earlier ability value for this key.

### Deferred Beyond This Plan

- Complete meaning and consumers of native reload field `Techno+0x200`.
- Complete writer/save/load/reset/removal inventory for native current ammo/readiness.
- Complete writer/save/load/reset/removal inventory for per-instance firepower.
- House combat modifier normal serialization and exhaustive reapplication lifecycle.
- Exact meaning of receiver argument slot 6.
- Exact conversion, if any, from arbitrary existing Rust `u16` veterancy values to
  native experience. Noncanonical values remain `LegacyOnly`.
- Final controller, expiration, detach, and last/final-reference relation API after
  the separate reference-authority session lands.
- Receiver wrapper, trigger, postlude, effect-helper, and concrete class gaps named in
  the July 13 G1/Task 2 reports.
- Every live reader/writer migration, snapshot/hash authority flip, and legacy-field
  removal. These require a new implementation plan after the gates above close.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Create | `src/rules/veterancy_abilities.rs` | Exact 18-byte ability arrays and corrected name mapping |
| Modify | `src/rules/mod.rs` | Export the rules value type |
| Modify | `src/rules/ini_parser.rs` | Preserve prior ability values for empty overlay reads |
| Modify | `src/rules/object_type.rs` | Parse full arrays and derive FEARLESS compatibility fields |
| Modify | `src/sim/movement/locomotor_tests.rs` | Initialize the new required ObjectType fields in the fixture |
| Modify | `src/sim/movement/teleport_movement.rs` | Initialize the new required ObjectType fields in the fixture |
| Create | `src/sim/entity_state/mod.rs` | Exact entity-state value types and shadow bundle |
| Create | `src/sim/entity_state/access.rs` | Focused view/mutation facade and fixed-size comparisons |
| Modify | `src/sim/mod.rs` | Export the simulation module |
| Modify | `src/sim/game_entity.rs` | Embed skipped shadow state and initialize it |
| Modify | `src/sim/world/world_spawn.rs` | Refresh the shadow at the centralized store boundary |
| Modify | `src/sim/world/mod.rs` | Rebuild the skipped shadow after snapshot load |
| Modify | `src/sim/snapshot.rs` | Add snapshot-neutrality/load-rebuild regression tests only |
| Modify | `src/sim/world/world_hash.rs` | Add hash-neutrality regression test only |

No task modifies `src/sim/world/techno_ai.rs`, live damage formulas, lifecycle
ordering, reference expiration, `SNAPSHOT_VERSION`, or golden hash constants.

## Interface Changes

New rules interfaces:

- `VeterancyAbility`
- `VeterancyAbilities`
- `ObjectType::veteran_abilities`
- `ObjectType::elite_abilities`

Existing rules behavior refined:

- `IniFile::merge` keeps the prior value when an overlay supplies an empty
  `VeteranAbilities` or `EliteAbilities` entry. Its signature and all unrelated-key
  behavior remain unchanged.

New crate-private simulation interfaces:

- `VitalityState`
- `ExperienceState` and `VeterancyRank`
- `ExperienceShadow`
- `ReadinessState`
- `CombatModifierState`
- `EntityRelations`
- `EntityStateShadow`
- `EntityStateView` and `EntityStateMut`
- `ShadowDiagnostic`, `ShadowComparisonClass`, `EntityStateFamily`, and
  `StateValue`
- `GameEntity::entity_state_view`
- `GameEntity::entity_state_mut`
- `GameEntity::rebuild_entity_state_shadow_from_legacy`

The legacy public fields remain unchanged, so existing gameplay consumers continue
to compile and behave identically. The new interfaces are crate-private until the
separate authority cutover.

## Sim Checklist

- [ ] No ordinary `f32`/`f64` gameplay state or arithmetic is added; native values
      use `NativeF32Bits`, `NativeF64Bits`, and `X87Chop53`.
- [ ] Shadow state is not folded into deterministic state hash in this phase.
- [ ] Shadow state is `#[serde(skip)]`; `SNAPSHOT_VERSION` remains 28.
- [ ] No dependency on render/ui/sidebar/audio/net is introduced.
- [ ] Tick ordering is unchanged; no live tick caller invokes comparison methods.
- [ ] `EntityStore` remains `BTreeMap<u64, GameEntity>` and iteration order is
      unchanged.
- [ ] Diagnostics use fixed-size enums and `&'static str`; no per-tick allocation is
      introduced.
- [ ] Lifecycle remains owned by `src/sim/world/lifecycle.rs`.

## Risk Areas

- `GameEntity` is serialized broadly. A missing `#[serde(skip)]` would silently change
  bincode layout and require a snapshot-version bump; neutrality tests are mandatory.
- Rebuilding the shadow before specialized state is populated would miss aircraft
  readiness. Refresh at `store_spawned_limbo`, not only in `GameEntity::new`.
- Treating arbitrary `u16` veterancy as `value / 100` would invent native writer
  semantics. Preserve it as `LegacyOnly`.
- Using raw bit ordering for native floats would mishandle negatives and special
  values. Rank queries must go through `X87Chop53::compare`.
- Replacing existing `veteran_fearless`/`elite_fearless` fields in this phase would
  broaden reader migration. Keep them as derived compatibility projections.
- Generic `IniSection::get_list` trims each token and `IniFile::merge` normally lets
  an empty patch replace a base value. Both differ from the verified ability parser:
  Task 1 must use raw comma-separated tokens and an ability-key-only empty-overlay
  exception.
- Adding relation writers now could collide with the separate reference-authority
  work. Initial relation state is observational only.
- Any direct snapshot/hash participation would violate the approved shadow-first
  sequence.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|---|---|---|---|
| 1 | Ability array has exactly 18 ordered bytes | Every ability consumer selects a native byte; a one-index shift changes combat, movement, vision, and survivability | Corrected Ghidra table plus parser tests for indices 0, 1, 2, 13, 17 |
| 1 | Non-empty list replaces the whole array; empty overlay preserves prior bytes | Clearing, appending, token trimming, or merging stale abilities changes downstream state | Base-plus-empty-overlay preservation, non-empty replacement, and interior-whitespace tests against `0x00477640`/`0x00528A10` |
| 2 | Vitality uses signed `i32` | Healing, overkill, negative values, and mutable damage writeback cannot be represented by `u16` | Boundary unit tests plus `0x005F5390` dword evidence |
| 2 | Experience preserves native f32 bits | Rank gates and future XP math consume native stored bits | Bit round trips and threshold-neighbor tests against `0x0074FF90`/`0x00750010` |
| 2 | Modifier defaults are exact double `1.0` bits | A different default changes every damage fold once authority lands | Assert `0x3ff0000000000000` for both fields |
| 3 | Comparison distinguishes representation gap from semantic divergence | Normalizing either class hides migration evidence | Fixed diagnostics tests for negative vitality, in-range mismatch, and uncomparable experience |
| 4 | Shadow cannot infer lifecycle from HP | Health and Object/Logic membership are separate native axes | Set shadow vitality negative and assert lifecycle/LogicVector unchanged |
| 4 | Shadow refresh happens after specialized construction and load | Stale readiness or relations would poison later comparisons | Finite-aircraft spawn and snapshot-rebuild tests |
| 5 | Shadow does not change snapshot bytes or world hash | Any leak changes saves/replays before authority approval | Byte-equality and hash-equality tests with divergent shadow values |

---

## Tasks

### Task 1: Add exact Veteran/Elite ability arrays

**Why:** The entity-state substrate needs exact rank-selected type data before an
exact experience state can be useful. The implementation remains rules-owned; the
two sim files receive compile-only test-fixture initializers and no gameplay change.

**Files:**

- Create: `src/rules/veterancy_abilities.rs`
- Modify: `src/rules/mod.rs`
- Modify: `src/rules/ini_parser.rs:138-145, 309-333`
- Modify: `src/rules/object_type.rs:28-33, 428-435, 862-905, 1056-1057`
- Modify: `src/sim/movement/locomotor_tests.rs:5-12, 112-122`
- Modify: `src/sim/movement/teleport_movement.rs:378-387, 485-497`

**Pattern:** Small rules value module plus compatibility projections on `ObjectType`.
This is a new exact-width pattern. The empty-overlay rule is a narrow, named exception
inside the existing INI merge; it must not change empty-value handling for unrelated
keys. The value parser consumes the raw merged string because generic `get_list`
trims interior tokens unlike native `strtok`.

**Step 1: Define the exact ordered value type**

Create `src/rules/veterancy_abilities.rs`:

```rust
//! Exact TechnoType VeteranAbilities / EliteAbilities byte arrays.
//!
//! A non-empty native INI read replaces all 18 bytes. Overlay preservation for
//! empty reads is handled by the INI merge before this value is constructed.

pub const VETERANCY_ABILITY_COUNT: usize = 18;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VeterancyAbility {
    Faster = 0,
    Stronger = 1,
    Firepower = 2,
    Scatter = 3,
    RateOfFire = 4,
    Sight = 5,
    Cloak = 6,
    TiberiumProof = 7,
    VeinProof = 8,
    SelfHeal = 9,
    Explodes = 10,
    RadarInvisible = 11,
    Sensors = 12,
    Fearless = 13,
    C4 = 14,
    TiberiumHeal = 15,
    GuardArea = 16,
    Crusher = 17,
}

impl VeterancyAbility {
    fn from_ini_name(name: &str) -> Option<Self> {
        // Native tokenization splits only on comma. Do not trim an interior
        // token: "FASTER, STRONGER" recognizes FASTER but not " STRONGER".
        Some(match name.to_ascii_uppercase().as_str() {
            "FASTER" => Self::Faster,
            "STRONGER" => Self::Stronger,
            "FIREPOWER" => Self::Firepower,
            "SCATTER" => Self::Scatter,
            "ROF" => Self::RateOfFire,
            "SIGHT" => Self::Sight,
            "CLOAK" => Self::Cloak,
            "TIBERIUM_PROOF" => Self::TiberiumProof,
            "VEIN_PROOF" => Self::VeinProof,
            "SELF_HEAL" => Self::SelfHeal,
            "EXPLODES" => Self::Explodes,
            "RADAR_INVISIBLE" => Self::RadarInvisible,
            "SENSORS" => Self::Sensors,
            "FEARLESS" => Self::Fearless,
            "C4" => Self::C4,
            "TIBERIUM_HEAL" => Self::TiberiumHeal,
            "GUARD_AREA" => Self::GuardArea,
            "CRUSHER" => Self::Crusher,
            _ => return None,
        })
    }
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct VeterancyAbilities([u8; VETERANCY_ABILITY_COUNT]);

impl VeterancyAbilities {
    pub fn from_ini_value(value: Option<&str>) -> Self {
        let mut bytes = [0; VETERANCY_ABILITY_COUNT];
        let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
            return Self(bytes);
        };
        for name in value.split(',') {
            if let Some(ability) = VeterancyAbility::from_ini_name(name) {
                bytes[ability as usize] = 1;
            }
        }
        Self(bytes)
    }

    pub const fn has(self, ability: VeterancyAbility) -> bool {
        self.0[ability as usize] != 0
    }

    pub const fn bytes(self) -> [u8; VETERANCY_ABILITY_COUNT] {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrected_native_indices_are_stable() {
        let flags = VeterancyAbilities::from_ini_value(Some(
            "FASTER,STRONGER,FIREPOWER,FEARLESS,CRUSHER",
        ));
        let bytes = flags.bytes();
        assert_eq!(bytes.len(), 18);
        assert_eq!(bytes[0], 1);
        assert_eq!(bytes[1], 1);
        assert_eq!(bytes[2], 1);
        assert_eq!(bytes[13], 1);
        assert_eq!(bytes[17], 1);
        assert_eq!(bytes.iter().filter(|&&value| value != 0).count(), 5);
    }

    #[test]
    fn constructor_default_is_zero_when_no_prior_value_exists() {
        assert_eq!(VeterancyAbilities::from_ini_value(None).bytes(), [0; 18]);
        assert_eq!(VeterancyAbilities::from_ini_value(Some("")).bytes(), [0; 18]);
    }

    #[test]
    fn unknown_names_do_not_shift_or_alias_known_indices() {
        let flags = VeterancyAbilities::from_ini_value(Some("UNKNOWN,ROF"));
        assert!(flags.has(VeterancyAbility::RateOfFire));
        assert_eq!(flags.bytes().iter().filter(|&&value| value != 0).count(), 1);
    }

    #[test]
    fn interior_token_whitespace_is_not_normalized() {
        let flags = VeterancyAbilities::from_ini_value(Some("FASTER, STRONGER"));
        assert!(flags.has(VeterancyAbility::Faster));
        assert!(!flags.has(VeterancyAbility::Stronger));
        assert_eq!(flags.bytes().iter().filter(|&&value| value != 0).count(), 1);
    }
}
```

**Step 2: Export the module**

Add to `src/rules/mod.rs` beside the other rules value modules:

```rust
pub mod veterancy_abilities;
```

**Step 3: Preserve prior ability values for empty overlays**

In `src/rules/ini_parser.rs`, add this narrow helper near the existing merge-policy
constants:

```rust
fn ability_key_preserves_on_empty_overlay(key: &str) -> bool {
    key.eq_ignore_ascii_case("VeteranAbilities")
        || key.eq_ignore_ascii_case("EliteAbilities")
}
```

In `IniFile::merge`, before `base_section.set(key, val)`, add:

```rust
// AbilityClass::ReadAbilities treats a zero-length ReadString result as
// absent and copies the prior 18-byte array.
if val.trim().is_empty() && ability_key_preserves_on_empty_overlay(key) {
    continue;
}
```

Do not generalize this to every empty INI value. Other readers have not been proven
to share the ability parser's fallback contract.

**Step 4: Store both arrays on ObjectType and preserve compatibility fields**

Import the new types in `src/rules/object_type.rs`:

```rust
use crate::rules::veterancy_abilities::{VeterancyAbilities, VeterancyAbility};
```

Add fields immediately before the existing FEARLESS compatibility fields:

```rust
/// Exact 18-byte VeteranAbilities array after base+md INI merge.
pub veteran_abilities: VeterancyAbilities,
/// Exact 18-byte EliteAbilities array after base+md INI merge.
pub elite_abilities: VeterancyAbilities,
```

At the start of `ObjectType::from_ini_section`, before `Self {`, calculate once:

```rust
let veteran_abilities =
    VeterancyAbilities::from_ini_value(section.get("VeteranAbilities"));
let elite_abilities =
    VeterancyAbilities::from_ini_value(section.get("EliteAbilities"));
```

Replace the current two independent list scans in the struct initializer with:

```rust
veteran_abilities,
elite_abilities,
veteran_fearless: veteran_abilities.has(VeterancyAbility::Fearless),
elite_fearless: elite_abilities.has(VeterancyAbility::Fearless),
```

Delete `ability_list_has`; no caller should remain.

**Step 5: Update existing ObjectType struct-literal fixtures**

In `src/sim/movement/locomotor_tests.rs`, import:

```rust
use crate::rules::veterancy_abilities::VeterancyAbilities;
```

In the `make_obj` literal, immediately before the existing FEARLESS projections, add:

```rust
veteran_abilities: VeterancyAbilities::default(),
elite_abilities: VeterancyAbilities::default(),
```

In the test module in `src/sim/movement/teleport_movement.rs`, add the same import and
the same two fields to the `make_drive_obj` literal. These are the only two direct
`ObjectType` literals in the current source tree; all other construction routes use
`ObjectType::from_ini_section`.

**Step 6: Add ObjectType parser regression tests**

Add alongside the existing fear parsing test:

```rust
#[test]
fn parses_full_veteran_and_elite_ability_arrays() {
    let ini = IniFile::from_str(
        "[E1]\nVeteranAbilities=FASTER,STRONGER,FEARLESS\n\
         EliteAbilities=SELF_HEAL,FIREPOWER\n",
    );
    let section = ini.section("E1").unwrap();
    let obj = ObjectType::from_ini_section("E1", section, ObjectCategory::Infantry);

    assert!(obj.veteran_abilities.has(VeterancyAbility::Faster));
    assert!(obj.veteran_abilities.has(VeterancyAbility::Stronger));
    assert!(obj.veteran_abilities.has(VeterancyAbility::Fearless));
    assert!(obj.elite_abilities.has(VeterancyAbility::SelfHeal));
    assert!(obj.elite_abilities.has(VeterancyAbility::Firepower));
    assert!(obj.veteran_fearless);
    assert!(!obj.elite_fearless);
}

#[test]
fn empty_ability_overlay_preserves_the_base_array() {
    let mut merged = IniFile::from_str(
        "[E1]\nVeteranAbilities=FASTER,STRONGER\nEliteAbilities=SELF_HEAL\n",
    );
    merged.merge(&IniFile::from_str(
        "[E1]\nVeteranAbilities=\nEliteAbilities=\n",
    ));
    let obj = ObjectType::from_ini_section(
        "E1",
        merged.section("E1").unwrap(),
        ObjectCategory::Infantry,
    );

    assert!(obj.veteran_abilities.has(VeterancyAbility::Faster));
    assert!(obj.veteran_abilities.has(VeterancyAbility::Stronger));
    assert!(obj.elite_abilities.has(VeterancyAbility::SelfHeal));
}

#[test]
fn nonempty_ability_overlay_replaces_the_whole_array() {
    let mut merged = IniFile::from_str(
        "[E1]\nVeteranAbilities=FASTER,STRONGER\n",
    );
    merged.merge(&IniFile::from_str(
        "[E1]\nVeteranAbilities=SELF_HEAL\n",
    ));
    let obj = ObjectType::from_ini_section(
        "E1",
        merged.section("E1").unwrap(),
        ObjectCategory::Infantry,
    );

    assert!(!obj.veteran_abilities.has(VeterancyAbility::Faster));
    assert!(!obj.veteran_abilities.has(VeterancyAbility::Stronger));
    assert!(obj.veteran_abilities.has(VeterancyAbility::SelfHeal));
}
```

**Step 7: Verify**

Before Cargo, check ownership:

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue |
    Select-Object ProcessName,Id,CPU
```

Run:

```powershell
cargo test -q veterancy_abilities
cargo test -q parses_full_veteran_and_elite_ability_arrays
cargo test -q empty_ability_overlay_preserves_the_base_array
cargo test -q nonempty_ability_overlay_replaces_the_whole_array
cargo test -q locomotor_tests
cargo test -q teleport_movement
```

Expected: every literal `test result:` line reports `ok`; existing FEARLESS tests stay
green, and all direct `ObjectType` literals compile with the new required fields.

### Task 2: Add exact entity-state value types and the shadow bundle

**Why:** Define representation and unresolved-state boundaries before any facade or
entity integration.

**Files:**

- Create: `src/sim/entity_state/mod.rs`
- Modify: `src/sim/mod.rs:23-34`

**Pattern:** New entity-owned value module. It reuses `native_x87` and contains no
gameplay formulas, tick logic, presentation imports, RNG, or collections.

**Step 1: Create the exact value module**

Create `src/sim/entity_state/mod.rs`:

```rust
//! Exact, non-authoritative entity-state shadow representations.
//!
//! Legacy GameEntity fields remain gameplay-authoritative in this phase. These
//! values are skipped by snapshots, omitted from world hashing, and accessed
//! through the sibling facade.

#![expect(
    dead_code,
    reason = "staged shadow interfaces are intentionally not live before writer migration"
)]

mod access;

pub(crate) use access::{
    EntityStateFamily, EntityStateMut, EntityStateView, ShadowComparisonClass,
    ShadowDiagnostic, StateValue,
};

use crate::sim::components::Health;
use crate::sim::intern::InternedId;
use crate::util::native_x87::{
    NativeF32Bits, NativeF64Bits, NativeX87Error, X87Chop53, X87Ordering,
};

const NATIVE_F32_TWO: NativeF32Bits = NativeF32Bits::from_bits(0x4000_0000);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VitalityState {
    pub current: i32,
    pub maximum: i32,
}

impl From<Health> for VitalityState {
    fn from(value: Health) -> Self {
        Self {
            current: i32::from(value.current),
            maximum: i32::from(value.max),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VeterancyRank {
    Rookie,
    Veteran,
    Elite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExperienceState {
    bits: NativeF32Bits,
}

impl ExperienceState {
    pub const ROOKIE: Self = Self {
        bits: NativeF32Bits::POSITIVE_ZERO,
    };
    pub const VETERAN: Self = Self {
        bits: NativeF32Bits::ONE,
    };
    pub const ELITE: Self = Self {
        bits: NATIVE_F32_TWO,
    };

    pub const fn from_bits(bits: NativeF32Bits) -> Self {
        Self { bits }
    }

    pub const fn bits(self) -> NativeF32Bits {
        self.bits
    }

    pub fn rank(self) -> Result<VeterancyRank, NativeX87Error> {
        let value = X87Chop53::load_f32(self.bits)?;
        let elite = X87Chop53::load_f32(NATIVE_F32_TWO)?;
        if matches!(
            X87Chop53::compare(value, elite),
            X87Ordering::Equal | X87Ordering::Greater
        ) {
            return Ok(VeterancyRank::Elite);
        }

        let veteran = X87Chop53::load_f32(NativeF32Bits::ONE)?;
        if matches!(
            X87Chop53::compare(value, veteran),
            X87Ordering::Equal | X87Ordering::Greater
        ) {
            return Ok(VeterancyRank::Veteran);
        }
        Ok(VeterancyRank::Rookie)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExperienceShadow {
    Exact(ExperienceState),
    LegacyOnly(u16),
}

impl ExperienceShadow {
    fn from_legacy(value: u16) -> Self {
        match value {
            0 => Self::Exact(ExperienceState::ROOKIE),
            100 => Self::Exact(ExperienceState::VETERAN),
            200 => Self::Exact(ExperienceState::ELITE),
            other => Self::LegacyOnly(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReadinessState {
    pub current: i32,
    pub maximum: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CombatModifierState {
    pub armor: NativeF64Bits,
    pub firepower: NativeF64Bits,
}

impl Default for CombatModifierState {
    fn default() -> Self {
        Self {
            armor: NativeF64Bits::ONE,
            firepower: NativeF64Bits::ONE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntityRelations {
    pub owner: InternedId,
    pub last_attacker_id: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntityStateShadow {
    pub(super) vitality: VitalityState,
    pub(super) experience: ExperienceShadow,
    pub(super) readiness: Option<ReadinessState>,
    pub(super) combat_modifiers: CombatModifierState,
    pub(super) relations: EntityRelations,
}

impl EntityStateShadow {
    pub(crate) fn from_legacy(
        health: Health,
        veterancy: u16,
        owner: InternedId,
        last_attacker_id: Option<u64>,
        readiness: Option<(i32, i32)>,
    ) -> Self {
        Self {
            vitality: health.into(),
            experience: ExperienceShadow::from_legacy(veterancy),
            readiness: readiness.map(|(current, maximum)| ReadinessState {
                current,
                maximum,
            }),
            combat_modifiers: CombatModifierState::default(),
            relations: EntityRelations {
                owner,
                last_attacker_id,
            },
        }
    }
}

impl Default for EntityStateShadow {
    fn default() -> Self {
        Self::from_legacy(Health { current: 0, max: 0 }, 0, InternedId::default(), None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vitality_projection_is_signed_and_lossless_for_legacy_health() {
        assert_eq!(
            VitalityState::from(Health {
                current: u16::MAX,
                max: u16::MAX,
            }),
            VitalityState {
                current: 65_535,
                maximum: 65_535,
            }
        );
    }

    #[test]
    fn exact_rank_thresholds_use_native_comparison() {
        assert_eq!(ExperienceState::ROOKIE.rank().unwrap(), VeterancyRank::Rookie);
        assert_eq!(
            ExperienceState::from_bits(NativeF32Bits::from_bits(0x3f7f_ffff))
                .rank()
                .unwrap(),
            VeterancyRank::Rookie
        );
        assert_eq!(ExperienceState::VETERAN.rank().unwrap(), VeterancyRank::Veteran);
        assert_eq!(ExperienceState::ELITE.rank().unwrap(), VeterancyRank::Elite);
    }

    #[test]
    fn noncanonical_legacy_experience_is_not_fabricated() {
        assert_eq!(
            ExperienceShadow::from_legacy(150),
            ExperienceShadow::LegacyOnly(150)
        );
    }

    #[test]
    fn modifier_defaults_are_exact_native_one_bits() {
        let modifiers = CombatModifierState::default();
        assert_eq!(modifiers.armor.bits(), 0x3ff0_0000_0000_0000);
        assert_eq!(modifiers.firepower.bits(), 0x3ff0_0000_0000_0000);
    }
}
```

**Step 2: Export the module**

Add to the core module block in `src/sim/mod.rs`:

```rust
pub(crate) mod entity_state;
```

**Step 3: Verify exact values only**

Run:

```powershell
cargo test -q entity_state::tests
```

Expected literal `test result:`: `ok`. No snapshot or world-hash test runs yet.

### Task 3: Add the focused shadow access and comparison facade

**Why:** Prevent new state from becoming another collection of directly mutated
fields and define divergence classification before embedding the bundle.

**Files:**

- Create: `src/sim/entity_state/access.rs`

**Pattern:** Borrowing facade over `GameEntity`; fixed-size returned diagnostics. New
pattern, modeled after the project's test-only lifecycle trace discipline but without
a global event vector or runtime allocation.

**Step 1: Define diagnostics and views**

Create `src/sim/entity_state/access.rs`:

```rust
//! Legacy-authoritative views, shadow mutations, and classified comparisons.

use crate::sim::components::Health;
use crate::sim::game_entity::GameEntity;

use super::{
    EntityRelations, EntityStateShadow, ExperienceShadow, ExperienceState,
    ReadinessState, VeterancyRank, VitalityState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntityStateFamily {
    Vitality,
    ExperienceRank,
    Readiness,
    Relations,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShadowComparisonClass {
    Equal,
    ExpectedRepresentationGap,
    SemanticDivergence,
    Uncomparable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StateValue {
    LegacyVitality { current: u16, maximum: u16 },
    ExactVitality { current: i32, maximum: i32 },
    LegacyExperience(u16),
    ExactExperienceBits(u32),
    Readiness { current: i32, maximum: i32 },
    Relations {
        owner: u32,
        last_attacker_id: Option<u64>,
    },
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShadowDiagnostic {
    pub tick: u64,
    pub entity_id: u64,
    pub operation: &'static str,
    pub family: EntityStateFamily,
    pub class: ShadowComparisonClass,
    pub legacy: StateValue,
    pub exact: StateValue,
}

pub(crate) struct EntityStateView<'a> {
    entity: &'a GameEntity,
}

impl<'a> EntityStateView<'a> {
    pub(super) fn new(entity: &'a GameEntity) -> Self {
        Self { entity }
    }

    pub(crate) fn exact_vitality(&self) -> VitalityState {
        self.entity.entity_state_shadow.vitality
    }

    pub(crate) fn compare_vitality(
        &self,
        tick: u64,
        operation: &'static str,
    ) -> ShadowDiagnostic {
        let legacy = self.entity.health;
        let exact = self.entity.entity_state_shadow.vitality;
        let representable = (0..=i32::from(u16::MAX)).contains(&exact.current)
            && (0..=i32::from(u16::MAX)).contains(&exact.maximum);
        let class = if !representable {
            ShadowComparisonClass::ExpectedRepresentationGap
        } else if exact == VitalityState::from(legacy) {
            ShadowComparisonClass::Equal
        } else {
            ShadowComparisonClass::SemanticDivergence
        };
        ShadowDiagnostic {
            tick,
            entity_id: self.entity.stable_id,
            operation,
            family: EntityStateFamily::Vitality,
            class,
            legacy: StateValue::LegacyVitality {
                current: legacy.current,
                maximum: legacy.max,
            },
            exact: StateValue::ExactVitality {
                current: exact.current,
                maximum: exact.maximum,
            },
        }
    }

    pub(crate) fn compare_experience_rank(
        &self,
        tick: u64,
        operation: &'static str,
    ) -> ShadowDiagnostic {
        let legacy = self.entity.veterancy;
        let legacy_rank = if legacy >= 200 {
            VeterancyRank::Elite
        } else if legacy >= 100 {
            VeterancyRank::Veteran
        } else {
            VeterancyRank::Rookie
        };
        let (class, exact_value) = match self.entity.entity_state_shadow.experience {
            ExperienceShadow::Exact(exact) => match exact.rank() {
                Ok(exact_rank) if exact_rank == legacy_rank => (
                    ShadowComparisonClass::Equal,
                    StateValue::ExactExperienceBits(exact.bits().bits()),
                ),
                Ok(_) => (
                    ShadowComparisonClass::SemanticDivergence,
                    StateValue::ExactExperienceBits(exact.bits().bits()),
                ),
                Err(_) => (ShadowComparisonClass::Uncomparable, StateValue::Missing),
            },
            ExperienceShadow::LegacyOnly(_) => {
                (ShadowComparisonClass::Uncomparable, StateValue::Missing)
            }
        };
        ShadowDiagnostic {
            tick,
            entity_id: self.entity.stable_id,
            operation,
            family: EntityStateFamily::ExperienceRank,
            class,
            legacy: StateValue::LegacyExperience(legacy),
            exact: exact_value,
        }
    }
}

pub(crate) struct EntityStateMut<'a> {
    entity: &'a mut GameEntity,
}

impl<'a> EntityStateMut<'a> {
    pub(super) fn new(entity: &'a mut GameEntity) -> Self {
        Self { entity }
    }

    pub(crate) fn mirror_vitality(&mut self, health: Health) {
        self.entity.health = health;
        self.entity.entity_state_shadow.vitality = health.into();
    }

    pub(crate) fn set_vitality_candidate(&mut self, candidate: VitalityState) {
        self.entity.entity_state_shadow.vitality = candidate;
    }

    pub(crate) fn mirror_experience(&mut self, legacy: u16) {
        self.entity.veterancy = legacy;
        self.entity.entity_state_shadow.experience = match legacy {
            0 => ExperienceShadow::Exact(ExperienceState::ROOKIE),
            100 => ExperienceShadow::Exact(ExperienceState::VETERAN),
            200 => ExperienceShadow::Exact(ExperienceState::ELITE),
            value => ExperienceShadow::LegacyOnly(value),
        };
    }

    pub(crate) fn set_experience_candidate(&mut self, candidate: ExperienceState) {
        self.entity.entity_state_shadow.experience = ExperienceShadow::Exact(candidate);
    }

    pub(crate) fn mirror_readiness_from_aircraft(&mut self) {
        self.entity.entity_state_shadow.readiness = self
            .entity
            .aircraft_ammo
            .as_ref()
            .map(|ammo| ReadinessState {
                current: ammo.current,
                maximum: ammo.max,
            });
    }

    pub(crate) fn mirror_relations(&mut self) {
        self.entity.entity_state_shadow.relations = EntityRelations {
            owner: self.entity.owner,
            last_attacker_id: self.entity.last_attacker_id,
        };
    }
}

impl GameEntity {
    pub(crate) fn entity_state_view(&self) -> EntityStateView<'_> {
        EntityStateView::new(self)
    }

    pub(crate) fn entity_state_mut(&mut self) -> EntityStateMut<'_> {
        EntityStateMut::new(self)
    }

    pub(crate) fn rebuild_entity_state_shadow_from_legacy(&mut self) {
        let readiness = self
            .aircraft_ammo
            .as_ref()
            .map(|ammo| (ammo.current, ammo.max));
        self.entity_state_shadow = EntityStateShadow::from_legacy(
            self.health,
            self.veterancy,
            self.owner,
            self.last_attacker_id,
            readiness,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_exact_health_is_a_representation_gap_without_legacy_mutation() {
        let mut entity = GameEntity::test_default(7, "E1", "Americans", 2, 3);
        let legacy = entity.health;
        entity.entity_state_mut().set_vitality_candidate(VitalityState {
            current: -1,
            maximum: i32::from(legacy.max),
        });
        let diagnostic = entity
            .entity_state_view()
            .compare_vitality(42, "test-negative-health");
        assert_eq!(diagnostic.class, ShadowComparisonClass::ExpectedRepresentationGap);
        assert_eq!(entity.health.current, legacy.current);
        assert!(entity.lifecycle.object_alive);
    }

    #[test]
    fn in_range_mismatch_is_semantic_divergence() {
        let mut entity = GameEntity::test_default(8, "E1", "Americans", 2, 3);
        let maximum = i32::from(entity.health.max);
        entity.entity_state_mut().set_vitality_candidate(VitalityState {
            current: 1,
            maximum,
        });
        assert_eq!(
            entity
                .entity_state_view()
                .compare_vitality(1, "test-in-range")
                .class,
            ShadowComparisonClass::SemanticDivergence
        );
    }

    #[test]
    fn noncanonical_legacy_experience_is_uncomparable() {
        let mut entity = GameEntity::test_default(9, "E1", "Americans", 2, 3);
        entity.entity_state_mut().mirror_experience(150);
        assert_eq!(
            entity
                .entity_state_view()
                .compare_experience_rank(1, "test-rank")
                .class,
            ShadowComparisonClass::Uncomparable
        );
    }
}
```

**Step 2: Verify the facade contract**

Run:

```powershell
cargo test -q entity_state::access::tests
```

Expected: literal `test result:` reports `ok`; the negative-health test proves no
lifecycle state changed.

### Task 4: Embed and rebuild the non-authoritative shadow

**Why:** Attach exact state to entity lifetime while proving that construction and
load rebuilding do not create a second live authority.

**Files:**

- Modify: `src/sim/game_entity.rs:18-45, 221-240, 591-705`
- Modify: `src/sim/world/world_spawn.rs:537-550`
- Modify: `src/sim/world/mod.rs:1395-1425`

**Pattern:** `#[serde(skip)]` derived shadow/cache rebuilt at centralized construction
and load boundaries. This mirrors existing occupancy/cache reconstruction while
remaining entity-owned.

**Step 1: Add the skipped field to GameEntity**

Import:

```rust
use crate::sim::entity_state::EntityStateShadow;
```

Add immediately after legacy `health`:

```rust
/// Exact candidate state for staged migration. Legacy fields remain live.
/// Skipped by snapshots and deliberately omitted from world hashing.
#[serde(skip)]
pub(crate) entity_state_shadow: EntityStateShadow,
```

In `GameEntity::new`, construct it before `Self`:

```rust
let entity_state_shadow =
    EntityStateShadow::from_legacy(health, veterancy, owner, None, None);
```

Insert `entity_state_shadow,` immediately after `health,` in the `Self` initializer.
Do not change the constructor signature or any caller.

**Step 2: Refresh at the centralized newly-stored-object boundary**

In `Simulation::store_spawned_limbo`, after the explicit lifecycle constructor facts
and before `self.substrate.entities.insert(ge)`, add:

```rust
ge.rebuild_entity_state_shadow_from_legacy();
```

This samples optional aircraft ammo after the spawn builders have populated it and
covers both `create_limbo` and `unlimbo` entry paths.

**Step 3: Rebuild after snapshot load**

In the existing entity loop at `Simulation::rebuild_caches_after_load`, keep screen
coordinate reconstruction and add:

```rust
for entity in self.substrate.entities.values_mut() {
    entity.position.refresh_screen_coords();
    entity.rebuild_entity_state_shadow_from_legacy();
}
```

Do not derive lifecycle facts, relation expiration, or LogicVector membership from
the shadow. Leave `rebuild_logic_membership` ordering unchanged.

**Step 4: Add constructor and finite-aircraft rebuild tests**

Add to `src/sim/game_entity.rs` tests:

```rust
#[test]
fn constructor_shadow_matches_legacy_without_owning_lifecycle() {
    let entity = GameEntity::test_default(1, "E1", "Americans", 3, 3);
    assert_eq!(
        entity
            .entity_state_view()
            .compare_vitality(0, "constructor")
            .class,
        crate::sim::entity_state::ShadowComparisonClass::Equal
    );
    assert!(entity.lifecycle.object_alive);
    assert!(entity.lifecycle.in_limbo);
    assert!(!entity.in_logic_vector);
}
```

Add this crate-private read method to `EntityStateView` in
`src/sim/entity_state/access.rs`:

```rust
pub(crate) fn exact_readiness(&self) -> Option<ReadinessState> {
    self.entity.entity_state_shadow.readiness
}
```

Then add this exact test to the existing `access.rs` test module. It exercises the
same rebuild method called by `store_spawned_limbo` after specialized construction:

```rust
#[test]
fn rebuild_samples_finite_aircraft_readiness_after_specialized_construction() {
    let mut entity = GameEntity::test_default(10, "ORCA", "Americans", 2, 3);
    entity.aircraft_ammo = Some(
        crate::sim::docking::aircraft_dock::AircraftAmmo::new(3),
    );
    entity.rebuild_entity_state_shadow_from_legacy();

    assert_eq!(
        entity.entity_state_view().exact_readiness(),
        Some(ReadinessState {
            current: 3,
            maximum: 3,
        })
    );
}
```

**Step 5: Verify construction only**

Run:

```powershell
cargo test -q constructor_shadow_matches_legacy_without_owning_lifecycle
cargo test -q rebuild_samples_finite_aircraft_readiness_after_specialized_construction -- --nocapture
```

Expected: both literal `test result:` lines report `ok`.

### Task 5: Prove snapshot and hash neutrality

**Why:** Shadow-first is only safe if the new bundle cannot alter save bytes,
snapshot compatibility, deterministic hashes, RNG, or lifecycle behavior.

**Files:**

- Modify tests only: `src/sim/snapshot.rs`
- Modify tests only: `src/sim/world/world_hash.rs`

**Pattern:** Existing snapshot rebuild tests and lifecycle hash provenance tests.

**Step 1: Add a snapshot round-trip rebuild test**

In the existing `src/sim/snapshot.rs` test module, add a test that:

1. builds the standard test simulation;
2. captures serialized bytes;
3. changes only `entity_state_shadow` through `set_vitality_candidate`;
4. serializes again and asserts byte equality;
5. deserializes and runs the existing `rebuild_load_caches` helper;
6. asserts the rebuilt shadow compares `Equal` to legacy vitality; and
7. asserts `SNAPSHOT_VERSION == 28`.

Use the existing `Simulation::new`, `GameEntity::test_default`,
`rebuild_load_caches`, and `flat_terrain` helpers exactly as follows:

```rust
#[test]
fn entity_state_shadow_is_snapshot_neutral_and_rebuilt_after_load() {
    use crate::sim::game_entity::GameEntity;

    let mut sim = Simulation::new();
    let entity_id = 1;
    sim.substrate.entities.insert(GameEntity::test_default(
        entity_id,
        "MTNK",
        "Americans",
        5,
        5,
    ));
    let before = bincode::serialize(&sim).expect("serialize before shadow mutation");

    sim.substrate
        .entities
        .get_mut(entity_id)
        .unwrap()
        .entity_state_mut()
        .set_vitality_candidate(crate::sim::entity_state::VitalityState {
            current: -123,
            maximum: 456,
        });

    let after = bincode::serialize(&sim).expect("serialize after shadow mutation");
    assert_eq!(before, after);

    let mut restored: Simulation = bincode::deserialize(&after).expect("deserialize");
    rebuild_load_caches(&mut restored, flat_terrain(16, 16));
    let entity = restored.substrate.entities.get(entity_id).unwrap();
    assert_eq!(
        entity
            .entity_state_view()
            .compare_vitality(0, "post-load-rebuild")
            .class,
        crate::sim::entity_state::ShadowComparisonClass::Equal
    );
    assert_eq!(super::SNAPSHOT_VERSION, 28);
}
```

**Step 2: Add a world-hash neutrality test**

In the existing `lifecycle_hash_tests` module in
`src/sim/world/world_hash.rs`, add:

```rust
#[test]
fn entity_state_shadow_is_not_hashed_before_cutover() {
    let mut sim = Simulation::new();
    let entity_id = 1;
    sim.substrate.entities.insert(GameEntity::test_default(
        entity_id,
        "MTNK",
        "Americans",
        5,
        5,
    ));
    let before = sim.state_hash();

    sim.substrate
        .entities
        .get_mut(entity_id)
        .unwrap()
        .entity_state_mut()
        .set_vitality_candidate(crate::sim::entity_state::VitalityState {
            current: i32::MIN,
            maximum: i32::MAX,
        });

    assert_eq!(sim.state_hash(), before);
}
```

Do not add any shadow field to `hash_entities`.

**Step 3: Verify neutrality**

Run serially:

```powershell
cargo test -q entity_state_shadow_is_snapshot_neutral_and_rebuilt_after_load -- --nocapture
cargo test -q entity_state_shadow_is_not_hashed_before_cutover -- --nocapture
cargo test -q lifecycle_authority -- --nocapture
```

Expected: every literal `test result:` line reports `ok`; version remains 28; no
golden constant changes.

### Task 6: Run the phase-boundary validation and inventory

**Why:** Confirm that the shadow foundation is additive and identify, without
migrating, the complete live-access surface for the next reviewed plan.

**Files:** No implementation files are changed in this task.

**Pattern:** Repository `rg` inventory plus focused tests, formatting, clippy, and one
final check. Do not edit findings merely to reduce counts.

**Step 1: Format only edited Rust files**

Run from the damage-authority worktree:

```powershell
rustfmt --edition 2024 `
  src/rules/veterancy_abilities.rs `
  src/rules/ini_parser.rs `
  src/rules/object_type.rs `
  src/rules/mod.rs `
  src/sim/movement/locomotor_tests.rs `
  src/sim/movement/teleport_movement.rs `
  src/sim/entity_state/mod.rs `
  src/sim/entity_state/access.rs `
  src/sim/mod.rs `
  src/sim/game_entity.rs `
  src/sim/world/world_spawn.rs `
  src/sim/world/mod.rs `
  src/sim/snapshot.rs `
  src/sim/world/world_hash.rs
```

Inspect `git diff --stat` and `git diff -- src/sim/world/techno_ai.rs`; the second
command must show no change from this work.

**Step 2: Re-run the access inventory**

```powershell
rg -n 'health\.current|health\.max' src/sim --glob '*.rs'
rg -n '\bveterancy\b' src/sim src/rules --glob '*.rs'
rg -n 'last_attacker_id' src/sim --glob '*.rs'
rg -n 'aircraft_ammo' src/sim --glob '*.rs'
```

Expected: direct legacy accesses still exist. Record their counts in the execution
handoff, not as a parity/completion ledger. Their presence is required evidence that
this phase did not silently flip authority.

**Step 3: Verify no forbidden authority changes**

```powershell
git diff -- src/sim/snapshot.rs src/sim/world/world_hash.rs
rg -n 'SNAPSHOT_VERSION' src/sim/snapshot.rs
rg -n 'entity_state_shadow' src/sim/world/world_hash.rs
```

Expected:

- `SNAPSHOT_VERSION` is still 28;
- `entity_state_shadow` occurs only in the new neutrality test, never in the hash
  implementation;
- snapshot changes are tests only;
- no lifecycle ordering code changed except the load-rebuild call in `world/mod.rs`.

**Step 4: Run focused and final verification serially**

First confirm no other session owns Cargo:

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue |
    Select-Object ProcessName,Id,CPU
```

Then run:

```powershell
cargo test -q veterancy_abilities -- --nocapture
cargo test -q entity_state -- --nocapture
cargo test -q snapshot -- --nocapture
cargo test -q world_hash -- --nocapture
cargo clippy -q --all-targets -- -D warnings
cargo check -q
```

Report every literal `test result:` line. Expected: all `ok`, clippy exits 0, and
`cargo check -q` exits 0.

**Step 5: Stop at the approved boundary**

Do not:

- change live combat `saturating_sub` sites;
- route live readers/writers through the facade;
- add a runtime diagnostic vector;
- modify last-attacker/reference cleanup;
- add general Techno readiness or reload timer fields;
- add House combat authority;
- serialize or hash the shadow;
- bump snapshot version or rebaseline goldens;
- remove legacy fields; or
- claim parity or completion.

The next action after this task is `/review-plan` on a new writer-migration plan after
the G1 research gates and overlapping reference/lifecycle ownership are reconciled.

## Sources & References

- **Approved design:**
  `docs/plans/2026-07-22-entity-state-authority-substrate-design.md`
- **Primary receiver synthesis:**
  `docs/research/DAMAGE_RECEIVER_CORE_REINVESTIGATION_2026-07-13.md`
  — synthesis complete, authority G1 failed; bounded signed/order claims verified,
  listed provenance gaps remain.
- **Rules/House/type assembly:**
  `docs/research/DAMAGE_RECEIVER_RULE_HOUSE_ASSEMBLY_REINVESTIGATION_2026-07-13.md`
  — partial, G1 failed; ability arrays and bounded armor/veterancy rows verified;
  readiness/firepower/House persistence gaps remain.
- **Concrete receiver/lifecycle reconciliation:**
  `docs/research/DAMAGE_CONCRETE_RECEIVER_REINVESTIGATION_2026-07-13.md`
  — partial, not authority-ready; class wrapper and lifecycle gaps remain.
- **Veterancy:** `docs/research/VETERANCY_SYSTEM_GHIDRA_REPORT.md`
  — verified report with corrected 18-entry ability mapping and explicit correction
  notes.
- **Damage multiplier order:**
  `docs/research/GATE_DAMAGE_COUNTRY_ARMOR_ORDER_RESOLUTION_GHIDRA_REPORT.md`
  — gate closed for bounded multiplier/order evidence, with July 13 correction
  superseding stale attacker prose.
- **Lifecycle synthesis:**
  `docs/research/OBJECT_TECHNO_LIFECYCLE_SHARED_STATE_SYSTEM_MODEL_SYNTHESIS.md`
  — high confidence for independent Object/Logic membership; broader Techno state
  remains incomplete.
- **Existing damage design:**
  `docs/plans/2026-06-04-damage-substrate-service-design.md`
- **Existing damage cutover plan:**
  `docs/plans/2026-07-13-damage-authoritative-cutover-plan.md`
- **Fresh live-binary checks during planning:**
  - `TechnoClass::ReceiveDamage @ 0x00701900`
  - `ObjectClass::ReceiveDamage @ 0x005F5390`
  - veterancy threshold checks `0x0074FF90`, `0x00750010`
  - ability-array parser `0x00477640`
  - zero-length `CCINIClass::ReadString @ 0x00528A10`
  - comma-only ability delimiter bytes at `0x00817F70`
  - per-instance multiplier fields `+0x158`, `+0x160`
  - readiness/current ammo field `+0x2FC`
- **INI keys:**
  - `ini/rulesmd.ini [General] VeteranRatio=3.0`
  - `ini/rulesmd.ini [General] VeteranCombat=1.1`
  - `ini/rulesmd.ini [General] VeteranArmor=1.5`
  - `ini/rulesmd.ini [General] VeteranCap=2`
  - object sections: `VeteranAbilities=`, `EliteAbilities=`, finite `Ammo=`
  - later authority only: `[AudioVisual] ConditionRed=25%`,
    `ConditionYellow=50%`; `[General] MaxDamage=10000`
- **Related code:**
  - `src/util/native_x87.rs`
  - `src/sim/game_entity.rs`
  - `src/sim/world/lifecycle.rs`
  - `src/sim/world/world_spawn.rs`
  - `src/sim/snapshot.rs`
  - `src/sim/world/world_hash.rs`
  - `src/sim/docking/aircraft_dock.rs`
  - `src/rules/object_type.rs`
  - `src/sim/map/bridge_occupancy_shadow.rs`
  - `src/sim/production/factory.rs`
- **Relevant commits:**
  - `95bef99d` — ordered lifecycle authority
  - `b5dbe09f` — pure shadow damage-math service
  - `768f760e` — stabilized current engine baseline

## Post-Plan Self-Review

- [x] The executable phase covers exact representations, arrays, facade, embedded
      shadow, load rebuild, and neutrality tests.
- [x] Authority flip, reader/writer migration, and unknown native fields are
      explicitly deferred rather than hidden behind placeholders.
- [x] Interfaces are defined before integration.
- [x] Snapshot, hash, lifecycle, native-float, and specialized-readiness risks each
      have named regression checks.
- [x] Every task identifies exact files, code, and commands.
- [x] No task imports render/ui/sidebar/audio/net into sim.
- [x] Research docs, fresh live-binary checks, repo patterns, and INI sources are
      cited.
- [x] Medium/low-confidence migration policy is flagged for `/review-plan`.
- [x] Parity-critical state widths, array ordering, thresholds, and neutrality are
      surfaced before tasks.
- [x] Review-plan blockers are closed: empty ability overlays preserve prior bytes,
      interior token whitespace is not normalized, and every direct `ObjectType`
      fixture initializes the new arrays.
- [x] No commits, staging, pushes, snapshot rebaseline, or implementation authority
      is implied.
