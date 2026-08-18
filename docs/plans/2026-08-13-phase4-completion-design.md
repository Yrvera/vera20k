# Phase 4 Completion Design

## Goal

Complete rows 53–79 of the clean-slate implementation order against active retail YR while preserving current mechanisms that still reproduce the verified contract.

## Architecture Context

Phase 4 is not one runtime module. Its 27 rows cross existing owners: `rules/` and `map/` construct retail data, `sim/` owns deterministic entity/lifecycle/scenario state, app code owns launch/input/presentation orchestration, and `render/`/`sidebar/`/`ui/` consume read-only presentation state. The earlier Phase 4 commits already placed behavior in those owners; later Phase 0, 1, and 3 work changed some of the same seams. The implementation therefore remains row-by-row and owner-local rather than introducing a new phase facade. [doc: `docs/gap-scans/2026-08-02-phase4/README.md`]

## Impact Analysis

- Reopen only a row whose current owner or evidence contract changed after its Phase 4 commit.
- Preserve `sim/` independence from app/render/UI layers and preserve live-object, RNG, lifecycle, snapshot, and tick-order authority.
- Treat snapshot schema, state hashes, scenario RNG consumption, draw order, clipping, palette choice, and frame/facing selection as high-risk seams.
- Validate one meaningful implementation batch at a time; run the full library suite once at phase completion.

## Chosen Approach

Use the existing Phase 4 order as the queue. For each row: establish the smallest active-retail contract from verified reports and narrow live checks; compare current code and later touches; have a builder preserve or correct the owner-local implementation; run the minimum non-interactive validation; then give a fresh read-only critic the requirement, evidence, current diff, and literal validation output. A row closes only after the critic passes it. This keeps verified behavior and avoids both a wholesale rewrite and stale “already done” assumptions.

## Player-Experience Detail Ledger

- `MILESTONE-BLOCKING`: selection admission must honor object life/limbo, ownership, and `Selectable=` without losing native selection order. [doc: `survey-GSI-05.05.md`; GHIDRA `ObjectClass::Select @ 0x005F4520`]
- `MILESTONE-BLOCKING`: passive houses must not prevent stock victory; passive acquisition must preserve native scan cadence and scenario-RNG ownership. [doc: `survey-GSI-05.16.md`, `contract-passive-acquire.md`]
- `MILESTONE-BLOCKING`: scenario construction/start must create active retail terrain and stock-start crates in native order. [doc: `survey-GSI-17.01.md`, `survey-GSI-01.04.md`]
- `MILESTONE-BLOCKING`: stock infantry, spawned missiles/aircraft, and per-building animations must have independent runtime state and native cadence. [doc: `survey-GSI-05.07.md`, `survey-GSI-05.08.md`, `survey-GSI-05.10.md`]
- `MILESTONE-BLOCKING`: shell/window/map-option/loading paths must admit stock data, freeze on focus loss, expose abort, and compose the correct selected/random-map presentation. [doc: `survey-GSI-01.01.md` through `survey-GSI-03.11.md`]
- `COMPOUNDING`: camera, layer/sort, translucency/depth, tile/overlay/SHP/voxel/light composition must preserve coordinate frame, facing block, palette, clipping, and draw-order contracts. [doc: `survey-GSI-13.01.md` through `survey-GSI-13.10.md`]
- `MILESTONE-BLOCKING`: end-of-match score/presentation must use shared simulation receipts and retail data rather than an invented presentation-only result. [doc: `survey-GSI-13.26.md`]
- `EXACTIFICATION-RESIDUAL`: a difference may remain only when its active trigger, frequency, player effect, determinism/architecture risk, and downstream consumers are established and a fresh critic agrees it is outside the ordinary stock Phase 4 gate. [doc: `docs/gap-scans/2026-08-02-phase4/README.md`]

## Design

### Components

No new subsystem. Changes stay with the current owning rule parser, map/scenario constructor, sim mechanism, app orchestrator, or renderer.

### Interfaces / Contracts

Rust structure remains native-independent; observable ordering, RNG draws, state bytes, lifecycle consequences, and retail INI/archive authority match verified gamemd behavior. New gamemd-derived code receives the canonical nearby provenance comment.

### Data Flow

Retail INI/archive data is parsed into existing rule/map types, scenario launch constructs authoritative sim state, deterministic ticks mutate it, and presentation layers consume snapshots without feeding gameplay behavior back into `sim/`.

### Error Handling

Use existing parser/application error paths. Do not add fallback values, clamps, or heuristics absent from the verified native path.

### Testing Strategy

Use current row-specific unit tests where they reproduce the contract; add only a missing end-to-end or boundary regression needed by a changed row. The orchestrator owns Cargo, captures literal output, and supplies it to a fresh critic. Run `cargo test -p vera20k --lib` exactly once at the final gate.

## Architectural Decisions

Follow existing deterministic owners and cross-layer boundaries. Do not add a Phase 4 abstraction or literal C++ class hierarchy. The only deviation from the 2026-08-02 disposition is evidence-driven reopening after later code touched an owning seam. No intentional technical debt is introduced.

## Alternatives Considered

- Wholesale Phase 4 rewrite: rejected because it would replace verified mechanisms, expand blast radius, and violate the request to preserve matches.
- Accept the prior Phase 4 completion wholesale: rejected because its own record contains refuted survey claims and later merged work changed shared owners.
- Group work by Rust directory instead of plan order: rejected because it would violate top-to-bottom row ownership and obscure cross-domain player contracts.

## Approval Record

The user explicitly authorized autonomous top-to-bottom implementation. Adversarial review found two load-bearing objections: stale survey claims could bless drift, and cross-domain batching could create architecture churn. Fresh per-row evidence/criticism resolves the first; owner-local edits in plan order resolve the second. Proceed.
