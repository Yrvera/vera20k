# GSI-04.04 Effective CellClass Tiberium View — Rejected Design

Date: 2026-07-24
Status: **REJECTED — DO NOT IMPLEMENT**
Approval: **NOT GRANTED**

## Outcome

Independent adversarial review rejected every approach that reconstructs an
"effective" LandType for selected readers while leaving the canonical Rust
cell state stale. The design phase is closed without a plan, branch, worktree,
or Rust edits.

## Approaches Considered

### A. Borrowed shared read view under `sim::tiberium`

The view would combine `OverlayGrid`, base terrain, overlay flags, and
tiberium rules for miner and reducer callers.

Rejected because it duplicates `CellClass::RecalcAttributes`, cannot reproduce
the exact Land byte without overlay Land/slope/removal inputs, and lets selected
callers disagree with the canonical resolved-terrain authority.

### B. Free function beside `OverlayGrid`

The function would compute an effective LandType/value on demand.

Rejected for the same semantic reason and because it couples generic overlay
storage to a partial tiberium-specific reconstruction.

### C. `Simulation` convenience method

The method would reach through the world facade to gather all inputs.

Rejected because it still creates shadow authority, expands an externally
owned dirty surface, and obscures which native writes remain deferred.

### D. Synchronous canonical recalculation

This is the only approach capable of preserving one authoritative cell byte
and same-tick visibility across all consumers. It is not a design alternative
inside this feature: it is the already-suspended GSI-04.04/04.06 dependency
and must retain ownership of terrain/zone recalculation.

## Challenge Findings

- Flagged outside-range overlays fall back to tiberium type index 0, not
  absence/value 0.
- Native classification includes `NumExtraImages`; stock TIB13–20 are active
  Riparius variants.
- Parsed overlay `Land` and slope/removal behavior make
  `Tiberium=yes ? 5 : base` non-equivalent to native recalculation.
- Current grid APIs can collapse dimension/registry unavailability into a
  known empty cell.
- The proposed mapper allocates and covers only the twelve flat variants.
- Helper-only tests would not prove any real production consumer.
- Reusing GSI-04.04 repeated both the stack row and the same authority owner,
  triggering the operator's dependency-cycle rule.

## Autonomous Decision

**Reject and cycle-pop.** The strongest objection cannot be repaired within a
read-only view: exactness requires writing the canonical cell state in native
order. Any repaired design would therefore become the existing synchronous
recalculation feature rather than this one.

The safe next dependency is exact `OverlayToTiberiumIndex` classification,
which is narrower and does not claim LandType ownership. It requires its own
contract, design challenge, plan review, feature branch, production-path
validation, and guarded integration.
