# Authoritative Damage Cutover Design

**Date:** 2026-07-13  
**Status:** Approved design; implementation not started  
**Scope:** Entity HP damage and negative-warhead healing routed through the
active Yuri's Revenge receiver mechanism. Service-depot repair, building repair,
selling, and other non-warhead healing mechanisms are excluded.  
**Supersedes for cutover work:**
`2026-06-04-damage-substrate-service-design.md` and
`2026-06-04-damage-substrate-service-implementation-plan.md`

The superseded documents remain useful historical context, but they contain
stale constants and code assumptions. In particular, the live binary uses 256
leptons per cell in the damage kernel, and stock rules set the running
`MaxDamage` value to 10000. This document is the authority for the next plan.

## Goal

Replace the scattered, simplified entity-health calculations with one ordered
damage receiver that preserves active `gamemd.exe` behavior. The cutover must:

- apply attacker modifiers exactly once;
- pass one signed incoming value and the per-target impact distance to the
  receiver;
- apply receiver modifiers, gates, falloff, Verses, caps, health transitions,
  and side effects exactly once;
- preserve event and target iteration order;
- keep current gameplay authoritative while the new path is compared in
  shadow mode; and
- refuse the authority flip until a retail-derived executable check proves the
  relevant numeric behavior.

Player-visible purpose: weapons, explosions, armor, veterancy, healing, and
special immunity interactions must remove the same health and cause the same
state transitions as Yuri's Revenge.

## Architecture Context

The current live combat path emits tuples shaped like
`(target_id, u16_damage, attacker_id, warhead_id)` and later subtracts the
already-calculated value with `saturating_sub`. Direct fire and area damage
calculate damage separately. Radiation and several superweapon/death paths have
additional calculations or direct health writes.

An additive service already exists under `src/sim/combat/damage/`. It contains a
useful separation between attacker math, receiver gates, the warhead kernel, and
health classification, but it is explicitly not wired into the live path. It is
scaffolding, not parity evidence. Its current use of Rust `f64` arithmetic and
`as i32` conversion does not prove x87/`Math__ftol` equivalence and therefore
cannot become authoritative unchanged.

The design stays inside the established layer boundary:

- `rules/` stores decoded rule inputs and validates references;
- `sim/combat/damage/` owns pure damage calculation over value views;
- `sim/combat/` owns ordered event construction and resolution;
- the resolver may emit simulation-side requests, but it never depends on
  render, UI, sidebar, audio, or networking; and
- `EntityStore` remains the owner of entities. The calculator does not reach
  into it.

## Verified Evidence Baseline

These facts were rechecked against the open retail `gamemd.exe` program before
this design was approved:

- `ApplyWarheadDamage` at `0x00489180` performs the damage-zero,
  scenario-no-damage, and null-warhead early outs; healing armor exclusion;
  CellSpread falloff; Verses multiplication; three `Math__ftol` conversions;
  and the signed `MaxDamage` cap.
- Memory at `0x007e2224` is `00 00 80 43`, IEEE-754 `256.0f`. Both
  `ApplyWarheadDamage` and `Apply_area_damage` at `0x00489280` use that value for
  their CellSpread-to-lepton conversion.
- `Apply_area_damage` passes the same post-attacker incoming damage to each
  target and passes each target's separately measured distance into virtual
  `ReceiveDamage`. It does not pre-apply the receiver kernel per target.
- The Rules constructor writes the missing-key fallback 1000 to
  `Rules+0x16C8`; the Rules INI reader loads `[CombatDamage] MaxDamage` into that
  field. Both stock `ini/rules.ini` and `ini/rulesmd.ini` set `MaxDamage=10000`,
  so standard YR runs with 10000.
- `MinDamage` is parsed but has no active Rules-field reader in the verified
  byte scan. It must not be introduced into the damage pipeline.
- `TechnoClass::ReceiveDamage` at `0x00701900` performs the defender armor
  divides and positive minimum-one step before the receiver immunity gates.
- `ObjectClass::ReceiveDamage` at `0x005f5390` invokes the warhead kernel,
  applies the building-only minimum-one rule, clamps overkill inclusively, and
  classifies the integer Yellow and floating Red crossings.
- Verses is an 11-element double array at the warhead and is multiplied before
  the final `Math__ftol`. Percent-form tokens use integer parsing before
  multiplication by 0.01; bare tokens use the native double parser.
- `ProneDamage` is stored legacy data but is not read by the active YR receiver
  path. The current Rust prone multiplier is drift and must not survive the
  authority flip.

### Constant-source conflict resolution

Two older gate reports are unsafe authorities for the constants they discuss:

- `GATE_DAMAGE_VERSES_F64_RESOLUTION_GHIDRA_REPORT.md` section D1c reports 128
  after misreading `0x43800000`; that bit pattern is 256.0f. The corrected
  authority is the live byte read plus
  `DAMAGE_MATH_GHIDRA_REPORT.md:880` and
  `combat/systems/splash_cellspread.md:94-100`.
- `GATE_DAMAGE_MAXDAMAGE_CLAMP_RESOLUTION_GHIDRA_REPORT.md` correctly exposes
  the constructor's 1000 missing-key fallback but incorrectly treats it as the
  standard stock runtime. The corrected stock authority is
  `ini/rules.ini:716` and `ini/rulesmd.ini:896`, both of which load 10000.

The verified correction is preserved in
`docs/research/DAMAGE_KERNEL_CONSTANTS_REVERIFICATION_2026-07-13.md`. An
implementation handoff must carry that correction bundle explicitly so the
research index cannot silently select the stale paragraphs. The stale values
must not appear in an executable plan or fixture.

Primary research references:

- `docs/research/DAMAGE_MATH_GHIDRA_REPORT.md`
- `docs/research/DAMAGE_KERNEL_CONSTANTS_REVERIFICATION_2026-07-13.md`
- `docs/research/RECEIVE_DAMAGE_GHIDRA_REPORT.md`
- `docs/research/RECEIVE_DAMAGE_PIPELINE_VERIFICATION_REPORT.md`
- `docs/research/combat/systems/damage_formula.md`
- `docs/research/combat/systems/splash_cellspread.md`
- `docs/research/GATE_DAMAGE_VERSES_F64_RESOLUTION_GHIDRA_REPORT.md` except its
  stale 128-lepton claim identified above
- `docs/research/GATE_DAMAGE_MAXDAMAGE_CLAMP_RESOLUTION_GHIDRA_REPORT.md` for
  constructor fallback and clamp mechanism, not stock runtime value
- `docs/research/GATE_DAMAGE_COUNTRY_ARMOR_ORDER_RESOLUTION_GHIDRA_REPORT.md`
- `docs/research/WARHEADTYPECLASS_REINVESTIGATION_GHIDRA_REPORT.md`

Where an older document disagrees with the rechecked bytes above, the live
binary and stock INI values win. Such a disagreement is not parity evidence.

## Chosen Approach

Use a structured transient damage event and one resolver. This was selected
over patching every formula independently or rewriting every health mutation
globally.

The central rule is:

> One damage event, one receiver calculation, one health update.

The event represents an attempt to call the native-shaped receiver. It does not
contain a precomputed final HP loss.

```rust
enum DamageWarheadRef {
    Resolved(InternedId),
    Null,
}

struct DamageEvent {
    target_id: u64,
    attacker_id: Option<u64>,
    source_house: Option<InternedId>,
    warhead_ref: DamageWarheadRef,
    incoming_damage: i32,
    distance_leptons: i32,
    ignore_defenses: bool,
}
```

This is an interface sketch, not committed source syntax. `source_house` is
separate from `attacker_id` because native area-damage calls can retain a source
house when no attacker object is present. `incoming_damage` is signed and is
already post-attacker-modifier damage; it has not passed through defender
armor, distance falloff, Verses, or the health clamp.

`Null` represents a deliberate runtime null warhead and reaches the verified
native early-out. It is not a substitute for a misspelled or unresolved INI
reference; unresolved content remains a rule-validation error.

The event is a typed call frame, not permission to defer damage to one global
phase. The authoritative resolver is invoked synchronously at the producer's
verified native call point, including once per AoE target during traversal.
Queuing is allowed only in shadow compatibility code or after a retail-derived
proof that the deferral preserves every same-tick read, write, removal, target
visit, trigger, and RNG consequence. Preserving vector insertion order alone is
not sufficient.

For area damage, target collection returns ordered location facts only:

```rust
struct AreaDamageTarget {
    target_id: u64,
    distance_leptons: i32,
}
```

The collector may filter and measure targets according to the verified
dispatcher. It must not calculate their final damage.

## Impact Analysis

### Rule data

Expected rule-layer touchpoints include:

- `src/rules/warhead_type.rs`: retain exact CellSpread, PercentAtMax, and Verses
  representations; add verified receiver-gate flags that are currently absent;
- `src/rules/object_type.rs`: expose verified type immunity and veteran ability
  inputs without inventing defaults;
- `src/rules/ruleset.rs`: add typed damage rules such as `MaxDamage`,
  `VeteranCombat`, `VeteranArmor`, and country armor/firepower multipliers; and
- existing legacy percentage fields remain temporarily for old readers but are
  retired from simulation consumers during the authority flip.

The constructor fallback and stock runtime value must remain distinct:
missing `MaxDamage` means 1000; stock YR INI overrides it to 10000.

### Simulation calculation

Expected calculation touchpoints are the existing files under
`src/sim/combat/damage/`. Their boundaries can be retained, but their numeric
implementation and incomplete input views must be replaced or extended before
authority.

### Event producers and health application

Expected producer/resolver touchpoints include:

- `src/sim/combat/mod.rs` for normal direct fire, current tuple emission, the
  Phase-4 subtraction, radiation events, and death processing;
- `src/sim/combat/combat_aoe.rs` for ordered target/distance collection without
  final damage math;
- death-weapon, radiation, and lightning-storm producers that currently
  calculate or apply entity HP damage independently, each resolved at its
  verified native tick phase rather than moved into a common later phase; and
- focused tests beside those systems.

The implementation plan must re-run `rg` and list every live caller by symbol;
line numbers in the superseded June plan are stale.

### Explicitly outside this cutover

- C4/sabotage remains on its verified mechanism-specific path until its bridge
  hut routing, `CanC4`, defenses, and return behavior have a dedicated adapter.
- Genetic Converter remains outside the authority flip until both native
  branches have verified adapters. Current Rust contains a guessed explosion
  constant in one branch and direct zero-health conversion in another; neither
  may be reclassified as ordinary damage by assumption.
- Bridge, wall, overlay, tiberium, and terrain-cell damage are not entity
  `ReceiveDamage` calls and must not be forced through this resolver.
- Service-depot and building repair are positive repair mechanisms, not
  negative-warhead `ReceiveDamage`, and remain with their current owners.
- Sell, crash, script removal, temporal removal, and other lifecycle writes to
  zero health are not automatically damage events.
- This design does not by itself certify all target enumeration and
  post-detonation effects inside `Apply_area_damage`. Any unproved dispatcher
  detail remains DRIFT or UNCHECKED even after receiver math improves.

## Design

### Components

#### 1. Exact rule inputs

Rule parsing stores the native-relevant numeric encodings rather than a lossy
gameplay percentage:

- `NativeF32Bits`-shaped values for CellSpread and PercentAtMax;
- `NativeF64Bits`-shaped values for each of the 11 Verses entries; and
- signed `i32` for `MaxDamage`.

Names are illustrative. The key contract is that the simulation receives the
original IEEE bit patterns and does not depend on `u8` percentages or ordinary
host floating-point operations.

Armor is validated at rule-load time into one of the 11 supported indices. A
missing or invalid armor value is a rule-validation error, not an unchecked
array index and not an invented fallback.

#### 2. Attacker calculation

A pure attacker stage accepts a value view of the firing entity, owner, weapon,
and relevant containment state. It applies only the verified `Fire_At` modifier
sequence and returns a signed `i32` incoming value. Garrison, tank-bunker, and
open-topped multipliers are applied here exactly once when their verified gates
are true.

Event producers that already receive post-attacker damage must mark that
contract explicitly so the resolver cannot apply attacker bonuses twice.

#### 3. Area target collector

The collector returns `AreaDamageTarget` entries in native traversal order. It
owns spatial inclusion, layer rules, building and aircraft distance adjustments,
and verified target filters. It does not own falloff, Verses, receiver gates, or
health mutation.

Before AoE authority, the collector must have executable fixtures for its
in-scope order and distance facts, including aircraft half-distance behavior.

#### 4. Pure receiver calculator

The receiver takes immutable value views:

- event input;
- target type and runtime state;
- attacker/source-house relationship state;
- decoded warhead data; and
- global/country damage rules.

It returns a `DamageOutcome` containing at minimum:

- signed final HP change (`> 0` is damage to subtract, `< 0` is healing to add);
- native receiver result state;
- a structured gate/block reason for shadow diagnostics; and
- verified simulation-effect requests that arise from the receiver.

The calculator never reads or writes `EntityStore`, consumes RNG, logs, renders,
or sends audio.

The outcome/result model includes every active return state, including
`PostMortem`. It cannot collapse native return codes to only
Unaffected/Damaged/Yellow/Red/Dead.

#### 5. Native receiver envelope

The central receiver is more than the numeric kernel. Its staged envelope owns
the following verified sequence and preserves early returns:

1. A concrete-class wrapper performs its native prechecks. For buildings this
   includes self-damage and Insignificant/UnsellableTransport gates.
2. `TechnoClass::ReceiveDamage` performs the defender modifier stages and
   positive minimum-one rule, then TypeImmune, Iron Curtain, warping-out, ammo
   absorption and ammo/animation mutation, bunker/force-shield handling,
   warhead immunities, ally gates, and psychedelic handling in the verified
   body order. `ignore_defenses` is tested at the individual native branches;
   it is not modeled as one blanket skip.
3. `ObjectClass::ReceiveDamage` performs its alive/damage-zero and
   `Insignificant` early outs, conditionally calls the kernel when defenses are
   not ignored, applies the building minimum, overkill assignment, health
   transition, kill credit, destruction marking, and trigger dispatch. The two
   first-damage event `0x29` calls remain two ordered calls.
4. `TechnoClass` aftermath for non-dead damage writes last-damage timing and
   distance, performs the verified flee/readiness behavior, creates or removes
   damage particle systems, and updates `WasAttacked` in native order.
5. The concrete-class wrapper processes the returned state. Building handling
   includes light dimming, damage sounds, RNG-consuming fire/spark creation,
   death cleanup/garrison behavior, attacker tracking, retaliation, and damage
   animation updates in the verified order.
6. `CausesDelayKill` building survival restores life/health state and returns
   `PostMortem` exactly where the building wrapper does so; it is not treated as
   an ordinary Dead result followed by a later repair.

The pure portion may calculate an ordered effect program over local value state,
but the synchronous resolver executes mutations, triggers, and RNG-consuming
effects at their native positions. If an effect's position or owner is not
verified, the affected concrete receiver class cannot enter the authority flip.

#### 6. Synchronous ordered resolver

The resolver processes each call immediately at its producer's verified phase.
For AoE it resolves one target before traversal advances to the next target. For
each call it snapshots the immutable views needed by the calculator, calls the
calculator once, and applies the result once. It owns integration side effects
such as health, condition-state refresh, fear, retaliation bookkeeping, kill
credit, triggers, and death/lifecycle work, but only in their verified order.

Each side effect needs an explicit parity status. A missing prerequisite blocks
the authority flip; it is not replaced with a plausible behavior.

#### 7. Shadow comparator

Before authority, the live result remains unchanged. The comparator evaluates
the new receiver from the same pre-application inputs and records a structured
difference containing the source, target, warhead, inputs, old result, and new
result. It must not panic, alter state, consume RNG, change ordering, or enter
the world hash.

Expected differences caused by known old drift are classified; they are not
silently treated as success.

Shadow migration uses a separate compatibility record:

```rust
struct ShadowDamageObservation {
    raw_call: DamageEvent,
    frozen_legacy_result: LegacyDamageResult,
}
```

The frozen legacy result is captured at each producer's current calculation cut
point and remains the only live value during shadow mode. The new receiver reads
`raw_call` from the pre-legacy facts. Direct fire, AoE, and radiation need
separate legacy adapters because they currently pre-apply different subsets of
Verses, falloff, and ProneDamage. A final `u16` legacy value must never be passed
back through the new receiver, which would apply those stages twice.

### Data Flow

```text
weapon or damage source
        |
        v
verified attacker modifiers, once
        |
        v
ordered DamageEvent(s) with raw signed damage and distance
        |
        v
synchronous resolver at the producer's native call point
        |
        v
pure receiver calculation, once
        |
        v
one health update plus immediate ordered simulation side effects
        |
        v
native-shaped death/lifecycle handling
```

For an explosion, every target receives the same incoming damage value but its
own measured distance, and each receiver call completes before the dispatcher
visits the next target. No producer may pre-apply the receiver kernel and then
call the full receiver again.

### Numeric Contract

Native damage math uses stored 32-bit and 64-bit IEEE values, x87 intermediate
operations, and explicit `Math__ftol` boundaries. Rust `f64` arithmetic followed
by `as i32` is not certified equivalent, especially for extended precision,
NaN, infinity, divide-by-zero, and out-of-range integer conversion.

The simulation therefore uses an integer-backed, deterministic implementation
of only the required native numeric operations. It must preserve:

- input IEEE bit patterns;
- x87 extended-precision operation order and comparison behavior used by the
  verified functions;
- the active rounding/control behavior at each `Math__ftol` call; and
- the native result for exceptional and boundary inputs accepted by the rules
  parser.

No additional clamp, zero guard, saturation, or normalization may be introduced
because it appears safer in Rust. If the accepted input domain can instead be
proved exhaustively equivalent under a smaller fixed representation, that proof
may replace software x87 emulation. Until one of those two routes passes a
retail-derived check, the numeric path remains UNVERIFIED and cannot be live.

### Error Handling

- Unresolved named warhead references and invalid armor references are caught
  during rules validation for normal content. They do not become indexing
  panics. A deliberate runtime `DamageWarheadRef::Null` is valid and follows the
  native zero-result early-out.
- A target absent when an ordered event resolves produces a deterministic
  `TargetGone` integration result and no health write. Shadow data must reveal
  this case so native live-vector/removal semantics can be checked.
- An absent attacker is valid. Source-house data remains available when the
  native caller supplies it.
- Unsupported special-effect prerequisites produce an explicit UNCHECKED/block
  status during development. They do not fall back to ordinary HP damage.
- Release gameplay never crashes merely because the shadow result differs.

### Determinism and Serialization

- Events are transient call frames and resolve synchronously at the verified
  producer phase; they are not independently serialized unless later evidence
  identifies a genuinely delayed native call.
- Event vectors preserve insertion order. No hash map or parallel commit may
  reorder them.
- The calculator consumes no RNG. Dispatcher-side RNG remains in its verified
  location and stream.
- Additive rule fields and shadow diagnostics must be state-hash neutral.
- The authority flip changes health/state outcomes and therefore takes the next
  free `SNAPSHOT_VERSION` at integration time. The current version is 25, but
  the design does not reserve or hardcode 26 because other tasks may merge
  first.

## Tiny-Detail Ledger

| Detail | Required contract | Status before implementation |
|---|---|---|
| Zero damage / scenario no-damage / null warhead | Native early-out order; deliberate null is representable | VERIFIED mechanism |
| Healing | Signed; special armor indices 8-10 cannot heal; no positive-only stages | VERIFIED mechanism |
| Cell unit | CellSpread multiplied by 256.0f | VERIFIED bytes |
| Falloff | Preserve native grouping, comparisons, and `ftol` boundary | VERIFIED mechanism; Rust numeric proof BLOCKED |
| PercentAtMax | Preserve native f32 value, not `u8` percent | Current Rust DRIFT |
| Verses | 11 f64 values, native parse branches, multiply then `ftol` | VERIFIED mechanism; current legacy readers lossy |
| MaxDamage | Signed field; fallback 1000, stock runtime 10000 | VERIFIED |
| MinDamage | Do not consume in active damage | VERIFIED dead read |
| Attacker modifiers | Apply verified `Fire_At` order once | Inputs incomplete in Rust |
| Defender modifiers | Country/unit armor divide, veteran divide, per-stage conversion | VERIFIED order; inputs incomplete |
| Defender minimum one | Positive damage floor before receiver gates | VERIFIED |
| TypeImmune | Evaluated after armor divides | VERIFIED |
| Other immunity gates | Preserve exact order and defaults | Some Rust fields missing; cutover BLOCKED |
| AffectsAllies | Use source relationship including attackerless source house | Adapter required |
| Ignore defenses | Test at each native branch; skip only the stages that body skips | Branch/callsite fixtures required |
| Ammo absorption | Mutate ammo and trigger depletion animation at its Techno receiver position | Missing from current service; BLOCKED |
| Insignificant | Preserve Building/Techno/Object wrapper early-outs and flag combinations | Missing from current service; BLOCKED |
| Building minimum one | Post-kernel, before zero handling, under verified building flag gate | VERIFIED mechanism |
| Overkill | Inclusive native comparison and remaining-HP assignment | Current Rust mechanism differs |
| Yellow state | Integer `Strength >> 1` crossing | VERIFIED |
| Red state | `Strength * ConditionRed` floating comparison | VERIFIED; exact numeric path required |
| PostMortem | `CausesDelayKill` building path restores HP=1/life and returns state 5 | Missing from current service; BLOCKED |
| Trigger sequence | Preserve every trigger, including both ordered event `0x29` calls | Integration owner/tests required |
| Damage aftermath | Timing/distance fields, flee, particle lifetime, and `WasAttacked` | Integration owner/tests required |
| Building wrappers | Prechecks, light/sound/fire RNG, death/garrison, tracking, retaliation, anims | Concrete receiver adapter required |
| ProneDamage | Never apply in active YR receiver | Current Rust DRIFT |
| AoE input | Same incoming damage for all targets, individual distance | VERIFIED |
| AoE order/distance | Native air/ground traversal and special distances | Must pass executable fixtures |
| Call timing | Resolve at the producer phase and per target before traversal continues | Current deferred paths need redesign/proof |
| Event order | Preserve producer and target order through each synchronous commit | Required deterministic invariant |
| Legacy shadow cut | Raw pre-receiver call and frozen current result remain separate | Per-producer adapters required |
| Healing cap to Strength | Preserve exact caller/receiver ownership and order | Recheck before plan task is executable |
| Fear/retaliation/kill credit | Apply once in verified receiver aftermath order | Integration trace required |
| Death timing | Queue/remove in native same-tick order | Integration trace required |
| C4 | Do not generalize into ordinary receiver without dedicated proof | OUT OF SCOPE |
| Genetic Converter | Verify guessed explosion and direct conversion branches separately | OUT OF SCOPE until adapter proof |
| Cell/bridge/terrain damage | Keep mechanism-specific paths | OUT OF SCOPE |
| Numeric equivalence | Retail-derived executable comparison or exhaustive proof | AUTHORITY BLOCKER |

## Testing Strategy

### 1. Rule parsing and validation

Test exact stored bits and native parse branches for CellSpread, PercentAtMax,
Verses, MaxDamage, armor indices, immunity flags, country modifiers, and veteran
abilities. Include missing-key fallback and stock merged-rules fixtures.

### 2. Pure calculation fixtures

Cover every stage and boundary separately:

- direct hit and multiple distances;
- each armor class and fractional Verses values;
- every `Math__ftol` boundary and a double-truncation divergence case;
- healing and special-armored healing rejection;
- MaxDamage below, equal to, and above the cap;
- defender and attacker modifier ordering;
- every immunity gate and alliance combination;
- deliberate null warhead versus unresolved named content;
- ammo absorption value/animation ordering and every `ignore_defenses` branch;
- Object/Techno/Building `Insignificant` early returns;
- building minimum-one, non-building zero, inclusive overkill;
- Yellow, Red, Dead, PostMortem, and negative-warhead healing outcomes;
- both first-damage `0x29` trigger calls and the other transition triggers;
- last-damage fields, flee/readiness, particles, `WasAttacked`, and building
  pre/post-wrapper effects; and
- exceptional numeric inputs accepted by the parser.

Hand-computed values are regression aids only. They do not certify parity.

### 3. Retail-derived Damage Oracle Fixture Set

The authority gate is a captured fixture set from the configured retail
`gamemd.exe`, recording inputs and outputs at:

- `ApplyWarheadDamage` (`0x00489180`);
- `TechnoClass::Fire_At` (`0x006fdd50`) for the attacker modifier stages;
- `Apply_area_damage` (`0x00489280`) for target order, filters, distance,
  per-target immediate commit timing, and later-target visibility;
- `TechnoClass::ReceiveDamage` (`0x00701900`); and
- `ObjectClass::ReceiveDamage` (`0x005f5390`); and
- `BuildingClass::ReceiveDamage` (`0x00442230`) for wrapper effects and
  `PostMortem`.

The fixture set must exercise the relevant stock and boundary matrix and retain
the raw numeric bits, signed integer results, return states, and ordered field
writes. Work on this probe must be coordinated with the separate Oracle task so
neither task edits or drives the same harness concurrently.

Passing Rust-vs-Rust tests or replay hashes is not a substitute for this gate.

### 4. Shadow integration

Feed old and new paths from the same pre-damage facts. Verify:

- shadow mode is hash-neutral and RNG-neutral;
- no event is lost, duplicated, or reordered;
- known prone, Verses, cap, and falloff differences are reported;
- no producer pre-applies the kernel; and
- each producer's raw call is compared with its frozen legacy result without
  feeding the legacy value through the new receiver; and
- missing targets and attackerless events are visible and deterministic.

### 5. In-scope producer integration

Test normal direct fire, weapon AoE, death weapons, radiation, and lightning
storm through the same resolver at each source's verified native phase. Include
same-tick fixtures where an earlier target dies or mutates state before a later
AoE target is visited. A route inventory/grep gate must prove no in-scope
producer still performs its own Verses/falloff/HP subtraction after the flip.
Genetic Converter is tested only after its separate native adapters are
verified and approved.

### 6. Final verification

Run focused tests serially, then one final `cargo check -q` after confirming no
other task owns Cargo. Format only edited Rust files. Coordinate the snapshot
version and golden rebaseline with all active tasks.

## Rollout

1. Add exact rule inputs and validation without changing live consumers.
2. Add typed raw call facts plus a separate frozen legacy-result adapter for
   each producer; retain the current authority and current application timing.
3. Correct/complete the attacker, full Object/Techno/concrete receiver envelope,
   result states, ordered effects, and exact numeric mechanism.
4. Separate AoE collection from final damage math and prove its in-scope order,
   filters, distances, per-target immediate commit timing, and same-traversal
   visibility.
5. Run the new calculation in read-only shadow mode for every in-scope producer.
6. Close all required rule/gate/side-effect inputs and pass the retail Damage
   Oracle Fixture Set.
7. In one coordinated authority change, invoke the resolver synchronously at
   every in-scope producer's verified phase, remove the legacy adapters and
   duplicate formulas including ProneDamage, take the next snapshot version,
   and update native-derived baselines.
8. Keep C4, Genetic Converter, repair, and other excluded mechanisms separate
   until their own verified adapters are approved.

Rollback is the focused authority-change commit: additive rule representations,
typed events, tests, and shadow diagnostics may remain if they are hash-neutral,
but the prior adapter can be restored without deleting research or fixtures.

## Architectural Decisions

### AD-1: One receiver owns final entity HP damage

This prevents the current direct/AoE/radiation/superweapon formulas from
drifting independently and prevents double application of the native kernel.

### AD-2: Events carry raw signed damage, not final `u16` loss

Healing, native signed caps, defender modifiers, and overkill cannot be
represented correctly by the current tuple.

### AD-3: Spatial collection and receiver math are separate

This mirrors the verified responsibility boundary: the area dispatcher measures
and orders targets; the receiver calculates each target's result.

### AD-4: Exact numeric proof is a hard authority gate

The existing f64 service may match common examples but has no exhaustive/native
proof. It remains UNVERIFIED until the software numeric path or an exhaustive
equivalence proof passes retail fixtures.

### AD-5: Shadow comparison never controls gameplay

Its purpose is evidence collection. A difference cannot crash a match, alter
state, or be hidden behind an assertion that disappears in release builds.

### AD-6: Mechanism-specific health writes are not bulk-refactored

Only verified calls equivalent to entity `ReceiveDamage` enter this cutover.
Lifecycle kills, cell damage, bridge damage, and C4 retain their owners.

### AD-7: Resolver placement follows native call timing

The shared receiver is a reusable function, not a new global damage phase. Each
producer invokes it where native YR invokes `ReceiveDamage`, and AoE commits one
target before continuing traversal.

### AD-8: Shadow inputs and legacy authority are separate

Every producer exposes raw pre-receiver facts for the new calculation while its
existing final result remains frozen for live application. This makes
double-application structurally visible and removable at cutover.

## Alternatives Considered

### Patch each existing formula in place

Rejected. It leaves multiple authorities and makes it easy for later producers
to omit a gate, conversion, or side effect.

### Replace every health write globally

Rejected. Many writes represent lifecycle transitions rather than weapon
damage. Treating them as ordinary receiver calls would change mechanisms and
same-tick behavior.

### Make the current f64 service authoritative immediately

Rejected. Common-case agreement is not proof of x87 and `Math__ftol` parity,
and several required rule/runtime inputs are still absent.

### Delay all structural work until every special warhead is decoded

Rejected. Typed events, exact rule storage, and read-only comparison are
reversible prerequisites that expose missing evidence without changing gameplay.
The final authority gate still requires every in-scope prerequisite.

## Completion Boundary

This design is complete when it has an implementation plan reviewed against the
then-current worktree. The feature is complete only when the retail-derived
fixture gate passes, all in-scope producers have one receiver authority, no
retired formula remains live, snapshot/hash changes are coordinated, and the
focused plus final checks pass.
