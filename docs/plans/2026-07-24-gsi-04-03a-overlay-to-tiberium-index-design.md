# GSI-04.03A Exact Overlay-to-Tiberium Classification Design

Date: 2026-07-24
Status: **APPROVED — AUTONOMOUS EVIDENCE GATE PASSED**
Contract:
`docs/contracts/2026-07-24-gsi-04-03a-overlay-to-tiberium-index-implementation-contract.md`

## Goal

Replace the current flat-art-family lookup with one allocation-free,
native-order type classifier that reproduces active
`CellClass::OverlayToTiberiumIndex @ 0x005FDD20`, while keeping verified
twelve-image flat placement unchanged.

## Architecture Context

`OverlayTypeRegistry` is the Rust owner of the compact runtime overlay ID
domain. It stores the ordered `[OverlayTypes]` names and the parsed flag record
for each ID. Its IDs already match the binary's section-entry ordinal model:
raw numeric INI keys are sorted as ordering keys and gaps do not reserve array
slots.

`TiberiumTypeRegistry` owns the ordered `[Tiberiums]` list. Each
`TiberiumTypeId` is its position in that list and each entry stores the parsed
`Image=` selector. That is the Rust-native equivalent of the ordered
`g_TiberiumClass_Array` and each class's image-selection state needed by the
native helper.

GSI-04.03B merge `b8cf64179b779275155cbcf0b5713a87ed589a8e`
now makes both immutable registries derive from the same retained
base+YR+selected-mode+bounded-map merged rules source. The synthetic `GASAND`
routing test and retail `MountMoras.map` rules-pass/overlay-no-op test passed.
Map-side allocation of brand-new type sections remains a separate residual.

The current `OverlayTypeRegistry::tiberium_overlay_mapping` mixes two
contracts:

- type classification for growth, spread, and reduction; and
- locating a zero-based variant inside the twelve primary flat-art images.

It constructs twelve formatted names and a `Vec` for every type on every
lookup. It recognizes no extra-image range, requires every flat sibling to be
present and flagged, and returns `None` for a flagged range miss.

Current type-only consumers are:

- `OreGrowthState::add_native_growth_queue_cell`;
- `OreGrowthState::rebuild_native_tiberium_queues_from_overlays`;
- `ore_growth::current_tiberium_type`, used by growth/spread processing;
- `sim::tiberium::current_tiberium_type`, used before shared reduction clears
  the overlay.

`flat_tiberium_variant_ids` has a separate valid role:
`place_native_spread_tiberium` uses it to reproduce flat germination's one RNG
choice among exactly twelve primary overlays. That art API must not inherit
the classifier's extra images.

Dependency flow after the change:

```text
OverlayTypeRegistry flags[id] ─┐
                               ├─> pure exact type classifier ─> TiberiumTypeId
ordered TiberiumTypeRegistry ──┘                                │
                                                                ├─> growth/spread
                                                                └─> reduction

OverlayTypeRegistry names ─> twelve-flat-art selector ─> placement overlay id
```

The mapping belongs in `map/overlay_types.rs`: that module already owns the
queried overlay ID and flag gate, while the ordered tiberium registry remains
a borrowed rules input. No new dependency crosses into render, UI, audio,
network, or world ownership.

## Operator Questions Resolved Autonomously

- Target: active standard Yuri's Revenge, including stock slope variants and
  direct map overlay bytes; not dormant TS-only growth semantics.
- Success criterion: exact helper result and corrected real production
  consumers, not merely family-name recognition.
- Priority: native result/order and deterministic RNG consequences first;
  allocation-free hot-path behavior second.
- Scope: classification only. The suspended synchronous
  RecalcAttributes/LandType dependency remains separate.
- Approval method: the operator specification replaces interactive selection
  with independent challenge, repair, and explicit autonomous approval.

## Impact Analysis

Changed implementation paths:

- `src/map/overlay_types.rs`
  - remove or retire the misleading flat-only mapping result;
  - add the exact type classifier and focused table/boundary tests;
  - retain the primary flat-art selector.
- `src/sim/ore_growth.rs`
  - replace three type-only mapping calls;
  - add a real native queue consumer test for an extra variant/fallback.
- `src/sim/tiberium/mod.rs`
  - replace the reduction-side type read;
  - strengthen the native spread-reseed test with an extra-image source.

Read-only dependencies:

- `src/rules/tiberium_type.rs`;
- `src/app_init.rs`;
- `src/sim/terrain_spawn.rs`;
- the corrected research reports and stock INIs.

Blast radius:

- Primary flat variants keep the same type IDs.
- TIB13-TIB20 and the corresponding TIB2/TIB3 extra ranges begin reaching the
  native queue/reduction class that Rust previously skipped.
- A flagged range miss begins reaching type 0. In a production consumer that
  queues work, this can restore a native RNG draw that Rust previously omitted;
  that is the intended deterministic correction.
- False-flag and absent/out-of-registry overlay IDs remain `None`.
- No serialized field, state-hash field, timer, queue representation, or
  entity order changes.
- No flat-placement RNG range changes.

Ownership risks:

- The three implementation paths are currently disjoint from the protected
  Inviso, GCLOCK2, combat, sidebar, and dirty damage-authority worktree paths.
  Exact Git/worktree state must still be reconciled before branch creation.
- `src/rules/tiberium_type.rs` is deliberately read-only in this feature; the
  unknown-owner damage worktree also carries rules-layer work and need not be
  intersected.

## Tiny-Detail Ledger

- No overlay and a queried ID absent from `OverlayTypeRegistry` produce no
  Rust type result. Native's exact `-1` test is at
  `0x005FDD24..0x005FDD2A`. [GHIDRA `0x005FDD20`]
- The queried overlay's own `Tiberium` byte is tested before class ranges.
  A false flag returns `-1` even if the numeric slot lies inside a stock range.
  [GHIDRA `0x005FDD35..0x005FDD42`]
- Classes are visited in `g_TiberiumClass_Array` order and the first matching
  class returns immediately. Duplicate `Image=` selectors therefore resolve to
  the first `[Tiberiums]` entry. [GHIDRA `0x005FDD4D..0x005FDD9B`]
- The primary comparison is half-open
  `[base, base + NumImages)`, not inclusive at the end.
  [GHIDRA `0x005FDD68..0x005FDD76`]
- The extra comparison is independently half-open
  `[base + NumImages, base + NumImages + NumExtraImages)`.
  [GHIDRA `0x005FDD81..0x005FDD91`]
- `NumImages` is 12 for the verified standard types.
  [doc: `OVERLAY_CLASS_SYSTEM_GHIDRA_REPORT.md` section 3a]
- `NumExtraImages` is 8 for Riparius, Vinifera, and Aboreus, and 0 for
  Cruentus. [GHIDRA `0x00721C5C..0x00721CD6`]
- Verified runtime bases are Riparius 102, Cruentus 27, Vinifera 127, and
  Aboreus 147. These are compact array slots, not raw INI keys.
  [GHIDRA `0x00721C5C..0x00721CD6`;
  `0x00668CF9..0x00668D32`]
- The parser's numeric-key gaps do not create runtime holes: GEM12 raw key 39
  is slot 38 and TIB18-TIB20 raw keys 122-124 are slots 119-121.
  [GHIDRA `0x00668CF9..0x00668D32`;
  doc: corrected `ORE_OVERLAY_SYSTEM_GHIDRA_REPORT.md` section 14]
- A flagged range miss is not absence. It follows the diagnostic path and
  returns integer 0. [GHIDRA `0x005FDDA4..0x005FDDC1`]
- Type 0 is the first standard `[Tiberiums]` entry, Riparius. Rust IDs are
  assigned from that ordered list. [ini: `rulesmd.ini [Tiberiums]`;
  Rust: `tiberium_type.rs:47-72`]
- `Get_Tiberium_Value` and `Reduce_Tiberium` reject only `-1`, so fallback 0
  has downstream state/value effects. [GHIDRA `0x0048502B..0x0048504F`;
  `0x00480B77..0x00480B82`]
- Standard sloped germination stores an extra-image overlay:
  `base + 12 + (slope-1)*2 + random(0,1)`.
  [GHIDRA `0x00487190`;
  doc: `CELLCLASS_PLACETIBERIUM_FUN_00487190_GHIDRA_REPORT.md` section 2.4]
- Flat germination remains `base + RandomRanged(0,11)`. Extra-image support in
  classification must not change that RNG bound or draw count.
  [GHIDRA `0x00487190`]
- Type classification reads no `OverlayData`, `ResourceNode`, terrain
  LandType, slope, object list, timer, or RNG. Those are consumer predicates,
  not helper inputs. [GHIDRA `0x005FDD20`]
- The query has no state write. It may restore later consumer work and RNG
  that was previously skipped, but it must introduce no independent draw or
  queue mutation. [GHIDRA helper body; Rust consumer trace]
- Integer comparisons operate in a widened type so `base + count` cannot wrap
  at `u8`. All verified standard endpoints fit below 256.
  [GHIDRA 32-bit integer arithmetic]
- A standard registry always has type id 0. Rust defensively returns `None`
  for an empty tiberium registry instead of indexing missing storage; that
  defensive case is outside the standard-YR parity claim.
  [ini: `rulesmd.ini [Tiberiums]`; Rust architecture safety]
- The verified fresh-registry selector table is complete for every
  representable `u8`: 2 selects Cruentus base 27/counts 12/0; 3 selects
  Vinifera 127/12/8; 4 selects Aboreus 147/12/8; every other value, including
  0, 1, 5, and 255, selects Riparius 102/12/8.
  [GHIDRA `0x00721C55..0x00721CD6`]
- Signed `-1` and stateful reread persistence are outside the current Rust
  `u8` representation and this bounded design.

## Approaches Considered

### A. Allocation-free native-order range query — chosen

How it works:

- Add a pure `OverlayTypeRegistry::tiberium_type_for_overlay` method.
- Reject an absent/false-flag queried overlay first.
- Iterate `TiberiumTypeRegistry::types()` in order.
- Translate each verified `Image=` selector to a small immutable range
  descriptor containing the native base, primary count, and extra count.
- Test the primary then extra half-open range and return that entry's id on the
  first match.
- After exhausting the list, return the first type's id (standard id 0).

Architectural fit:

- Keeps the ID/flag read with the current overlay registry owner.
- Borrows the existing ordered tiberium registry; no duplicate parsed state or
  construction-order change.
- Matches the native loop directly without recreating global pointers.

Tiny-detail coverage:

- flag order lives at method entry;
- class order lives in slice iteration;
- both half-open checks live in one range descriptor helper;
- compact bases/counts are named binary-derived constants;
- fallback uses the first ordered type only after loop exhaustion;
- flat placement continues using `flat_tiberium_variant_ids`;
- no allocations, state writes, or RNG occur in the method.

Trade-offs:

- Performs at most four tiny range checks for stock data, exactly the same
  asymptotic/native iteration shape.
- Binary-derived runtime bases are explicit constants. That is intentional:
  native `ReadINI` selects those array slots; deriving them from family names
  would change behavior when names/siblings are altered.
- Every representable `u8` selector follows the verified complete switch.

Touched files:

- `src/map/overlay_types.rs`;
- the type-only consumer callsites in `src/sim/ore_growth.rs` and
  `src/sim/tiberium/mod.rs`.

Risk areas:

- accidentally checking ranges before the queried flag;
- using raw INI keys instead of compact IDs;
- returning hardcoded id 0 when the Rust type registry is empty;
- widening flat placement to twenty images;
- keeping the old flat-variant mapping API alive as a second authority.

### B. Immutable precomputed per-overlay result table — rejected

How it works:

- Build a `Vec<Option<TiberiumTypeId>>` aligned with overlay runtime IDs and
  make every query O(1).

Architectural fit problem:

- The answer depends on both registries. They now derive from the same retained
  merged source, but no persistent paired-registry/cache owner exists.
- Storing the table in `OverlayTypeRegistry` requires signature changes across
  unrelated render/map constructors or a hidden lazy cache keyed by the first
  borrowed tiberium registry.
- Storing it in `TiberiumTypeRegistry` requires reparsing/duplicating overlay
  flags and compaction.
- A standalone paired context adds new plumbing to every consumer for a
  four-entry native loop.

Parity fit:

- A correctly built table can preserve outputs, but first-match collisions,
  fallback 0, and registry identity become construction invariants rather than
  visible query order.
- A `OnceLock` inside one registry would be wrong if the same overlay registry
  is queried with multiple rule fixtures: the first caller would silently own
  later answers.

Tiny-detail coverage:

- All ledger items are representable, but a table still needs a new persistent
  paired immutable owner and explicit rebuild semantics.

Trade-offs:

- Saves at most four range checks while adding state, construction coupling,
  tests for cache identity, and broader callsite churn.
- More opportunity for a stale derived table without a measurable standard
  workload benefit.

Verdict:

- Rejected for this feature. It is Rust-native but materially less transparent
  than the native-order pure query and introduces unnecessary authority.

### C. Expand the current family-name mapper to twenty variants — rejected

How it works:

- Generate TIB/GEM names 1-20, search IDs, and reuse the current mapping result.

Parity failure:

- Native uses compact runtime ranges selected by `Image=`, not successful
  lookup of every expected filename.
- Requiring all siblings makes one missing/false-flag sibling invalidate an
  otherwise valid queried slot.
- A flagged stray still needs fallback type 0.
- Cruentus has no extra range.
- Reordering/duplicate selectors no longer follow the first native class.

Verdict:

- DRIFT. It fixes one stock sample but does not implement the verified helper.

## Chosen Approach

Use approach A: one allocation-free, borrowed, native-order range query in
`OverlayTypeRegistry`.

The proposed public shape is:

```text
OverlayTypeRegistry::tiberium_type_for_overlay(
    &self,
    tiberium_types: &TiberiumTypeRegistry,
    overlay_id: u8,
) -> Option<TiberiumTypeId>
```

The exact helper names may follow local conventions during planning, but these
contracts are frozen:

- return value is only the native type id; there is no synthetic flat variant;
- the queried flag gate precedes range reads;
- type slice order is authoritative;
- 2, 3, and 4 select their verified descriptors and every other `u8` selects
  the Riparius descriptor;
- primary and extra comparisons remain distinct and half-open;
- flagged exhaustion returns the first registered type;
- empty type registry returns `None` defensively;
- the method allocates nothing and mutates nothing.

`TiberiumOverlayMapping` and its `flat_variant` field have no remaining
type-only consumer. The plan should remove them and the misleading
`tiberium_overlay_mapping` API unless an actual art consumer is found during
final impact search. `flat_tiberium_variant_ids` remains the sole primary-art
selection API.

## Design

### Components

`NativeTiberiumOverlayRange` (private, copyable)

- `base: usize`
- `primary_count: usize`
- `extra_count: usize`
- predicates for the two half-open ranges, or one total-range predicate only
  if tests still pin the primary/extra boundary separately.

`native_tiberium_overlay_range(image: u8)`

- returns 27/12/0 for 2, 127/12/8 for 3, and 147/12/8 for 4;
- returns the Riparius descriptor 102/12/8 for every other `u8`;
- is exhaustive and therefore does not return `Option`.

`OverlayTypeRegistry::tiberium_type_for_overlay`

- owns the flag gate and ordered search;
- returns the first type id or standard fallback id 0;
- has no cache, allocation, mutation, RNG, or sim dependency.

### Interfaces / Contracts

- `OverlayTypeRegistry` remains constructed from the retained merged rules
  source plus optional art data.
- `TiberiumTypeRegistry` remains constructed and ordered exactly as now.
- No constructor signature changes.
- `ore_growth` callsites receive `TiberiumTypeId` directly instead of a
  mapping wrapper.
- `sim::tiberium` receives the same direct id before overlay clearing.
- Flat placement still requests `[u8; 12]` from
  `flat_tiberium_variant_ids`.

### Data Flow

```text
overlay id
  │
  ├─ flags[id] unavailable/false ───────────────> None
  │
  └─ true
      │
      └─ ordered tiberium types
          ├─ first primary/extra range match ───> that TiberiumTypeId
          └─ no match ──────────────────────────> first type / id 0
```

Consumers then execute their already-owned predicate/queue/reduction logic.
The classifier itself never reads or writes a cell.

### Error Handling

- Unknown/out-of-range overlay ID: `None`.
- Known overlay with `Tiberium=false`: `None`.
- Known flagged overlay, nonempty standard registry, no matching range:
  first ordered type id.
- Empty tiberium registry: `None` defensive result; no panic or fabricated id.
- Every representable `u8` `Image=` selector has a verified descriptor. Signed
  `-1` and stateful rereads remain outside this Rust representation.
- Native diagnostic logging is not gameplay state. A warning may be emitted on
  flagged exhaustion, but it must not add mutable dedupe state or affect the
  returned id.

### Testing Strategy

In `src/map/overlay_types.rs`:

1. Build one realistic stock-order fixture with raw numeric keys starting at 1
   and gaps at 40/41. Fill intervening entries so runtime slots are real rather
   than hand-assigned.
2. Assert compact IDs and every primary/extra boundary for all four types.
3. Assert a false-flag in-range slot returns `None`.
4. Assert a flagged outside-range slot returns id 0 and the same false-flag
   slot returns `None`.
5. Assert duplicate `Image=` selectors keep the first type.
6. Exhaustively assert all 256 selectors: 2/3/4 use their dedicated
   descriptors and every other value uses Riparius.
7. Assert unknown overlay IDs and an empty tiberium registry return `None`.
8. Retain/adjust the flat-art test to prove it still returns exactly primary
   variants 1-12.
9. Replace the old compressed 0-based family fixture with the stock-shaped
   compact fixture; it cannot serve as exact-classifier evidence.

In `src/sim/ore_growth.rs`:

10. Rebuild the compressed fixture into stock-shaped runtime slots through at
    least TIB2_20.
11. Exercise the real map-load
    `rebuild_native_tiberium_queues_from_overlays` method with TIB2_20 under
    otherwise-valid gates. Assert only class 2 receives work. This method has
    no RNG input and seeds zero priorities.
12. Exercise `add_native_growth_queue_cell` separately and compare its final
    RNG state with a clone advanced by exactly one raw draw after successful
    classification and gates.

In `src/sim/tiberium/mod.rs`:

13. Rebuild the compressed reduction fixture into stock-shaped runtime slots.
14. Use TIB2_20 or TIB3_20 as the removed overlay and an eligible same-type
    neighbor. Assert only the corresponding nonzero class is reseeded and
    compare final RNG state with a clone advanced once per accepted neighbor
    in verified neighbor order. This pins the reduction-side consumer without
    changing the separately blocked reducer threshold or recalc behavior.

Validation must also run the existing overlay-type tests, focused native
tiberium queue tests, focused shared reducer tests, and `cargo check -q`.

## Architectural Decisions

- Follow the existing borrowed-registry query pattern instead of adding
  derived simulation state.
- Preserve the native class-order walk because it is already allocation-free
  and bounded to four stock types.
- Use named binary-derived runtime-slot constants. The native mechanism
  hardcodes those array slots; replacing them with family-name completeness
  checks is less exact, not more data-driven.
- Separate type identity from art variant identity. This removes the current
  hidden coupling and keeps flat RNG selection visibly twelve-wide.
- Do not cache. A cache would need a new paired-registry authority and
  invalidation/identity contract that the current immutable parse flow does not
  need.
- Introduce no serialized or hashed data and no new module.

## Strongest Rejection Argument

The design hardcodes standard runtime base slots and therefore could appear to
violate the project's rule against hardcoding data that belongs in INI files.

Response:

These bases are not visual filenames or tunable INI values. The active binary's
`TiberiumClass::ReadINI` switch turns `Image=` into pointers at exact
`g_OverlayTypeClass_Array` slots 102, 27, 127, and 147, while the list loader
compacts entries by ordinal. Those slots are the verified mechanism. Rebuilding
them by requiring names TIB01-GEM12 would change behavior for incomplete or
reordered lists and recreate the rejected flat-family shortcut. The constants
are therefore binary-derived control data, named and tested against a parsed
stock-order fixture, while values and flags continue to come from INI.

## Autonomous Approval Gate

**APPROVED.** GSI-04.03B resolved the source seam and merged as `b8cf6417`.
Independent source-impact review, document challenge, and live Ghidra
spot-check then found and resolved three load-bearing issues:

- `SpazWH` is a Warhead and cannot block this overlay classifier;
- every representable selector other than 2/3/4 uses Riparius;
- class-0 consumer fixtures were vacuous and must use TIB2/TIB3 nonzero
  classes.

The live binary challenge is GREEN for this bounded immutable-registry design.
The strongest remaining rejection case—hardcoded runtime bases—does not hold
because those slots are outputs of the native selector switch, not tunable art
data. Non-load-bearing residuals are signed `Image=-1`, stateful rereads,
full selected-mode native equivalence, and map-side allocation of brand-new
types. Implementation is authorized only in the three explicitly owned Rust
paths after a separately reviewed implementation plan.
