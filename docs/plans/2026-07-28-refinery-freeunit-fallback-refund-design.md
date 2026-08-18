# Refinery FreeUnit Fallback and Refund Design

## Goal

Complete the verified stock-refinery failure path so an occupied primary bay
cannot receive an overlapping FreeUnit, fallback attempts reuse one constructed
unit, and total placement failure cleans up and refunds the owner exactly once.

## Architecture Context

Phase 9 already owns the stable-ID-ordered `BuildingUp -> None` transition and
passes completed refinery IDs to
`production_refinery::spawn_completed_refinery_free_units`. That production
module resolves the data-driven `FreeUnit`, while `Simulation` owns construction,
limbo storage, Reveal, occupancy, owner counts, pending deletion, and wallets.

`Simulation::try_reveal_entity` is already result-bearing:
`PlacementEvidence::MarkFailed` restores limbo without registering occupancy or
logic, while `MarkSucceeded` commits the coordinates, occupancy, display output,
and logic membership. `Simulation::uninit` releases owner counts, conceals any
committed state, and queues physical deletion. These are the existing
transaction boundaries; the refinery code should supply policy and evidence
rather than duplicate lifecycle mutations.

The active YR path at `gamemd.exe:0x00445F80` constructs one UnitClass, attempts
the primary cell with facing `0xC0`, performs two ordered nearby-placement
attempts with facing `0xA0`, and on total failure refunds the owner-adjusted unit
cost before uninitializing the constructed object. Stock `CMIN` and `HARV` cost
1400. The trace
`docs/research/traces/REFINERY_FREEUNIT_FALLBACK_REFUND_TRACE_20260728.md`
confirmed the Rust overlap/refund divergence at commit `799515ca`.

## Impact Analysis

- `src/sim/production/production_refinery.rs`
  owns the FreeUnit transaction, primary admission exception for the source
  refinery, bounded fallback candidates, refund, and cleanup.
- `src/sim/world/world_spawn.rs`
  already exposes limbo construction and does not need a new generic spawn mode.
- `src/sim/world/lifecycle.rs`
  remains unchanged; production consumes its existing Reveal and UnInit APIs.
- `src/sim/production/production_placement_tests.rs`
  gains occupied-primary/fallback and total-failure/refund acceptance tests.

State hashes intentionally change only when the trigger occurs: fallback
coordinates/facing, wallet credits, owner count, occupancy, and stable-ID state
are deterministic consequences of the existing Phase-9 completion order.

## Chosen Approach

Use a production-owned transaction over existing generic lifecycle authority:

1. Construct the configured unit once in limbo.
2. Attempt the primary cell using caller-computed placement evidence that ignores
   only the source refinery's expected bay occupancy and rejects every other
   ground-layer occupant.
3. On rejection, attempt at most two distinct deterministic compatibility
   fallback candidates with facing `0xA0`, reusing the same stable ID.
4. On success, leave Reveal as the only occupancy/logic commit owner.
5. On total failure, add the resolved unit cost to the owner wallet, then call
   `uninit`; the normal same-tick pending-delete drain removes the object.

This is preferred over widening every generic spawn caller: only the FreeUnit
path currently has the verified source-building overlap exception and
constructed-object retry/refund contract.

## Player-Experience Detail Ledger

- **MILESTONE-BLOCKING:** the primary `(rx+2,ry+2)` remains admissible when its
  only ground occupant is the completing refinery.
  `[GHIDRA 0x00445F80; trace stage 2]`
- **COMPOUNDING:** any independent ground occupant makes primary Reveal fail;
  Rust must never insert a second non-infantry entity over it.
  `[GHIDRA 0x00445F80; trace stages 3 and 8]`
- **COMPOUNDING:** fallback uses the same constructed stable ID, exactly two
  ordered attempts, and facing `0xA0`.
  `[GHIDRA 0x00445F80; trace stage 4]`
- **COMPOUNDING:** total failure refunds before cleanup; stock refund is 1400,
  owner counts return to their pre-attempt value, and no limbo residue survives.
  `[GHIDRA 0x00445F80; rulesmd.ini CMIN/HARV Cost=1400; trace stages 6-7]`
- **EXACTIFICATION-RESIDUAL:** the one native boolean differing between the two
  `Find_Nearby_Passable_Cell` calls and its exact returned cells remain
  unverified. The existing deterministic perimeter candidate order is retained
  as an explicitly non-certified selector. Trigger: occupied primary bay.
  Player effect: a successful miner may appear at a different nearby cell.
  It does not permit overlap, duplicate construction, missing cleanup, or a
  wrong refund.
  `[trace stage 9: UNCHECKED; implementation contract BLOCKED row]`
- **EXACTIFICATION-RESIDUAL:** mission-10 Assign/Commence timer equivalence
  remains unverified. Existing Harvest initialization is preserved.
  `[implementation contract mission row]`

## Design

### Components

`production_refinery` adds:

- a small placement-attempt helper that updates the limbo object's facing,
  derives the candidate height, and calls `try_reveal_entity` with
  `MarkSucceeded` or `MarkFailed`;
- a primary admission predicate that filters occupancy by layer and ignores
  exactly the source building ID;
- a deterministic iterator returning up to two distinct compatibility fallback
  cells from the existing perimeter order;
- a total-failure helper that credits `ObjectType.cost.max(0)` and invokes
  lifecycle `uninit`.

### Interfaces / Contracts

- One eligible completion allocates at most one unit stable ID.
- Failed Reveal leaves that stable ID alive and in limbo for the next attempt.
- Only successful Reveal sets cell marking or logic membership.
- Missing/invalid `FreeUnit` data is a no-op and is never refunded.
- Refund occurs only after actual construction followed by exhausted placement.
- `spawned_entities` is true only if a placement commits successfully.

### Data Flow

```text
completed refinery
  -> resolve FreeUnit + cost
  -> construct once in limbo
  -> primary admission / Reveal(C0)
  -> fallback candidate 1 / Reveal(A0)
  -> fallback candidate 2 / Reveal(A0)
  -> success
     OR refund -> UnInit -> normal pending-delete drain
```

### Error Handling

Missing rules objects or invalid coordinates skip construction. Once
construction succeeds, exhaustion of all representable/admitted attempts is a
real placement failure and receives exactly one refund and cleanup. Arithmetic
remains checked; no coordinate clamps or wraps are introduced.

### Testing Strategy

- Occupied primary with an available fallback: no overlap, exactly one miner,
  fallback facing `0xA0`, same completion tick.
- Primary plus all fallback cells unavailable: `spawned_entities=false`,
  credits increase by 1400 once, no living/limbo miner remains, owner unit count
  is unchanged, and later ticks do not refund again.
- Existing completion timing, primary cell, Allied/Soviet identity, and stable
  ordering tests remain green.
- Run only scoped `--lib` production tests while this branch is active; the
  merge owner runs the full `cargo test -p vera20k --lib` exactly once.

## Architectural Decisions

- Follow the existing split: production decides admission/candidates/refund;
  lifecycle alone commits Reveal and UnInit state.
- Reuse the existing limbo constructor and result-bearing Reveal API instead of
  adding a FreeUnit-only entity store path.
- Do not turn generic `spawn_object` into a policy-rich placement API; its other
  callers do not share the source-refinery exception.
- Preserve the exact fallback selector as an honest residual rather than
  inventing semantics for the undecoded native option.

## Alternatives Considered

### Widen generic `spawn_object` with placement policy callbacks

This could make every caller result-bearing, but it spreads a refinery-specific
co-occupancy exception across unrelated map, production, deploy, and test
callers. The blast radius is unnecessary for the verified slice.

### Create a new FreeUnit-specific lifecycle implementation

Directly mutating positions, occupancy, logic membership, owner counts, and
deletion from `production_refinery` would duplicate established authority and
risk partial cleanup. Rejected.

### Wait for exact fallback-cell decoding

This would leave overlapping entities and missing refunds in ordinary congested
bases. Exact candidate selection is separable: the verified transaction,
attempt count, facing, cleanup, and refund can close now without falsely
certifying the residual selector.
