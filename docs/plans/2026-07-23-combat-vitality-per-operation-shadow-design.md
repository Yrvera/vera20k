# Combat Vitality Per-Operation Shadow Design

## Goal

Compare the live Phase-4 combat HP writer with the verified signed Object vitality
base transaction once per damage event, without changing gameplay authority.

## Architecture Context

`sim/combat/mod.rs` collects positive damage events and later applies them in one
ordered Phase-4 loop with `u16::saturating_sub`. The live result then drives fear,
building-state refresh, under-attack notifications, retaliation, and batched death
handling.

`sim/combat/damage/object_vitality.rs` now provides a pure signed Object vitality
transaction. `sim/entity_state` owns a signed, non-authoritative candidate and
classified legacy-versus-exact diagnostics. The candidate is excluded from
serialization and world hashing.

## Impact Analysis

- `rules/object_type.rs`: parse the active Object `Immune=` input.
- `rules/ruleset.rs`: expose the original `ConditionRed` double bits.
- `sim/entity_state/access.rs`: construct explicit `Uncomparable` vitality rows.
- `sim/combat/mod.rs`: collect one transient diagnostic per Phase-4 event.
- Focused combat and parser tests validate the seam.

The live subtraction, event order, fear, building refresh, under-attack behavior,
retaliation, death batching, RNG, lifecycle, snapshots, and hashes remain unchanged.

## Chosen Approach

Each event starts from its current legacy pre-write vitality. The pure signed
transaction computes an independent candidate using the event damage as a
conditional post-normalization value. The existing live subtraction then runs
unchanged. The candidate is stored in the skipped shadow, compared with legacy
health, and returned through a crate-visible `Vec<ShadowDiagnostic>` on
`CombatTickResult`.

Rebasing each operation from current legacy state prevents unrelated unmigrated
writers and earlier mismatches from cascading. An `Equal` row proves only that the
legacy HP write matches the Object base transition for the supplied normalized
amount. It does not certify producers, projectile timing, the damage kernel, Techno
gates, callbacks, or lifecycle.

Lethal `Cyborg=yes` Infantry is `Uncomparable` because the verified one-time rescue
requires instance state and callbacks not present in the bounded pure transaction.
Native-x87 input failures are also `Uncomparable`.

## Tiny-Detail Ledger

- Phase-4 events remain in their existing vector order.
  [Rust: `src/sim/combat/mod.rs`, Phase 4]
- Entry Health and the candidate transaction are signed dwords.
  [doc: `DAMAGE_RECEIVER_CORE_REINVESTIGATION_2026-07-13.md` §3.3 O1]
- Entry Health `<= 0` and requested damage `== 0` preserve the request and HP.
  [same doc, O2]
- Object `Immune` gates defenses-active damage before kernel/vitality mutation.
  [same doc, O2]
- Building `CanC4=false` floors the post-kernel value to positive one.
  [same doc, O4]
- Negative healing preserves signed writeback, caps to Strength, and requests
  callback argument 7 only when HP changes.
  [same doc, O6]
- Positive result begins at numeric result 1.
  [same doc, O7]
- Yellow uses signed `Strength >> 1`, not `ConditionYellow`.
  [same doc, O8]
- Overkill comparison and damage writeback are inclusive.
  [same doc, O9]
- Red uses strict x87 comparisons against `Strength * ConditionRed` and overwrites
  Yellow.
  [same doc, O10]
- HP commit precedes callback/death continuation.
  [same doc, O11]
- Lethal custom Cyborg Infantry requires one-time rescue state and is not silently
  treated as comparable.
  [same doc, O12; stock INI activation absent]
- Exact zero is a fatal receiver candidate, not an UnInit command.
  [doc: `DAMAGE_SIGNED_VITALITY_WRITER_AND_FATAL_HANDOFF_GHIDRA_REPORT.md` §§6,12]
- Callback, kill-credit, destruction, and wrapper-lifecycle intents are never
  executed by this slice.
  [same doc, §12.3]
- Shadow candidate and diagnostics remain outside snapshots and hashes.
  [same doc, §12.1]

## Design

### Components

- `ObjectType::immune`: exact typed rule input, default false.
- `GeneralRules::condition_red_native_bits()`: crate-visible exact-bit accessor.
- `EntityStateView::uncomparable_vitality(...)`: typed diagnostic constructor.
- `CombatTickResult::vitality_shadow_diagnostics`: transient crate-visible rows.
- Phase-4 adapter: builds the pure transaction input, preserves the live writer,
  stores the candidate, and classifies the result.

### Interfaces / Contracts

The Phase-4 adapter uses `DamageNormalization::KernelWriteback(damage as i32)` only
as a conditional post-normalization candidate. Its operation name must retain that
scope so downstream status reports cannot claim full receiver parity.

Every event that reaches the existing HP write produces exactly one diagnostic.
Invulnerability-nullified and missing-target events do not invoke that writer and
therefore produce no writer diagnostic. A successful pure transaction uses
`compare_vitality`; unsupported Cyborg or x87 rows use `Uncomparable` with the legacy
value retained and exact value marked missing.

### Data Flow

1. Snapshot current legacy vitality and typed target facts.
2. Detect unsupported lethal Cyborg input.
3. Otherwise compute the pure signed Object vitality candidate.
4. Run the existing legacy subtraction and all existing immediate consumers.
5. Store only the candidate, then compare and append the diagnostic.
6. Continue existing death collection and later phases unchanged.

### Error Handling

No production unwraps. Unsupported mechanisms and native-x87 errors become
`Uncomparable`. Missing object rules also become `Uncomparable`; they do not invent
defaults beyond the parser's verified `Immune=false` default.

### Testing Strategy

- `Immune=` parser default and explicit-value tests.
- Exact `ConditionRed` bit accessor test.
- Ordinary positive and inclusive-overkill events compare Equal.
- Object-immune damage reports SemanticDivergence while live HP remains unchanged
  from its prior behavior.
- Lethal custom Cyborg damage reports Uncomparable.
- Multiple events emit diagnostics in application order.
- Diagnostics do not alter existing death/lifecycle outputs.

## Architectural Decisions

The behavior remains in `sim/combat/damage`; `entity_state` only represents and
compares state. The live writer remains authoritative. Per-operation rebasing is a
diagnostic isolation policy, not the future authority model.

No new persistent state, serializer field, hash input, RNG use, lifecycle call, or
cross-layer dependency is introduced.

## Alternatives Considered

- Accumulating exact shadow across events was rejected because unmigrated writers
  and missing receiver-tail mechanisms would create cascading false mismatches.
- Producer-side comparison was rejected because it runs before the actual ordered HP
  write and cannot observe the correct entry Health.
- Log-only diagnostics were rejected because they are hard to test and aggregate;
  the user approved transient typed rows.
