# GSI-04.03A Exact Overlay-to-Tiberium Classification Implementation Plan

Status: **APPROVED — REVIEW-PLAN READY**

> **For Codex:** Execute task-by-task in the dedicated feature worktree. Own
> only the three listed Rust paths, validate the real queue/reduction paths,
> and use the guarded no-commit integration process.

**Goal:** Replace the flat-art-only type lookup with the exact immutable
`CellClass::OverlayToTiberiumIndex` result while leaving twelve-image flat
placement unchanged.

**Architecture:** `OverlayTypeRegistry` owns the queried compact overlay ID and
flag gate. It borrows the ordered `TiberiumTypeRegistry`, derives the native
fresh-construction range descriptor from each parsed `Image: u8`, walks in
type order, and returns the first match or the first type for a flagged miss.
Simulation consumers receive only `TiberiumTypeId`; art selection keeps its
separate twelve-primary-image API.

**Contract:**
`docs/contracts/2026-07-24-gsi-04-03a-overlay-to-tiberium-index-implementation-contract.md`

**Design:**
`docs/plans/2026-07-24-gsi-04-03a-overlay-to-tiberium-index-design.md`

## Grounding Summary

- Live `gamemd.exe` `0x005FDD20` gates on the queried overlay's `Tiberium`
  byte, scans ordered primary then extra half-open ranges, returns the first
  class index, and returns 0 for a flagged miss.
- Live `0x00721A50` gives the complete fresh-construction selector table:
  `2 => 27/12/0`, `3 => 127/12/8`, `4 => 147/12/8`, and every other
  representable `u8 => 102/12/8`.
- Runtime bases are compact `[OverlayTypes]` slots, not raw numeric keys.
- GSI-04.03B merge `b8cf6417` already routes the same bounded merged rules
  source to `RuleSet` and `OverlayTypeRegistry`.
- `flat_tiberium_variant_ids` remains valid for native flat germination's
  exactly twelve primary images.
- Signed `Image=-1`, stateful rereads, map-side new-type allocation, and
  synchronous cell-recalculation authority remain outside this slice.

## Owned Paths

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/map/overlay_types.rs` | Exact classifier, range descriptor, removal of conflated mapping wrapper, and exhaustive focused tests. |
| Modify | `src/sim/ore_growth.rs` | Direct type-id consumers, stock-shaped fixture, production rebuild and exact RNG tests. |
| Modify | `src/sim/tiberium/mod.rs` | Direct reduction consumer, stock-shaped fixture, nonzero-class reseed and exact RNG test. |

Read-only paths include `src/rules/tiberium_type.rs`, `src/app_init.rs`,
`src/app_init_helpers.rs`, `src/sim/terrain_spawn.rs`, and every protected
combat/sidebar/rules/world surface.

## Task 0: Freeze evidence and create the feature worktree

1. Require root `dev` to be clean at the recorded validated baseline.
2. Record branches, worktrees, stashes, active agents, Cargo/rustc processes,
   dirty ownership, contract/design hashes, and exact base SHA in the
   operational journal.
3. Create a unique
   `feature/gsi-04-03a-overlay-classifier-<run-suffix>` branch and linked
   `<local>/Documents/ra2-rust-game-gsi-04-03a-<run-suffix>` worktree.
4. Require feature HEAD to equal the recorded base and the new worktree to be
   clean.
5. Copy the main checkout's ignored `ini/` directory into the worktree because
   compile-time `include_str!` users require it even though classifier tests
   construct stock-shaped registries in memory. Resolve both paths first,
   require the destination to remain inside the worktree and not be a reparse
   point, record the physical copy, never stage it, and remove that exact copy
   safely before worktree cleanup.

## Task 1: Add the exact pure classifier

**File:** `src/map/overlay_types.rs`

1. Remove public `TiberiumOverlayMapping` and
   `tiberium_overlay_mapping`; final impact search must show no consumer needs
   `flat_variant`.
2. Add named native constants and a private copyable descriptor:

   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   struct NativeTiberiumOverlayRange {
       base: usize,
       primary_count: usize,
       extra_count: usize,
   }
   ```

3. Add a `const fn new` (or use struct literals) and named constants for bases
   and counts. Add an exhaustive private selector helper:

   ```rust
   fn native_tiberium_overlay_range(image: u8) -> NativeTiberiumOverlayRange {
       match image {
           2 => NativeTiberiumOverlayRange::new(27, 12, 0),
           3 => NativeTiberiumOverlayRange::new(127, 12, 8),
           4 => NativeTiberiumOverlayRange::new(147, 12, 8),
           _ => NativeTiberiumOverlayRange::new(102, 12, 8),
       }
   }
   ```

4. Use widened `usize` half-open comparisons so endpoint arithmetic cannot
   wrap at `u8`.
5. Add:

   ```rust
   pub fn tiberium_type_for_overlay(
       &self,
       tiberium_types: &TiberiumTypeRegistry,
       overlay_id: u8,
   ) -> Option<TiberiumTypeId>
   ```

   It must:

   - return `None` if the overlay ID is unknown or its own flag is false;
   - preserve ordered type iteration;
   - check primary then extra half-open ranges;
   - return the first matching type id;
   - return `tiberium_types.types().first().map(|ty| ty.id)` after a flagged
     miss, handling an empty registry without panic;
   - allocate nothing, mutate nothing, and consume no RNG.

6. Preserve `flat_tiberium_variant_ids` unchanged as the art API.

## Task 2: Replace the classifier unit fixture and pin every edge

**File:** `src/map/overlay_types.rs`

1. Replace the compressed `0..47` mapping test with a stock-shaped fixture:

   - emit raw keys `1..=170`, skipping 40 and 41;
   - place exact family names at the stock raw keys;
   - use unique filler names elsewhere;
   - flag the queried tiberium overlays explicitly.

2. Assert exact compact IDs and classification endpoints:

   - GEM01/GEM12 at 27/38 -> class 1;
   - explicitly flag slot 39 and assert fallback class 0, proving Cruentus's
     half-open end rather than false-flag rejection;
   - TIB01/TIB12/TIB13/TIB20 at 102/113/114/121 -> class 0;
   - TIB2_01/TIB2_12/TIB2_13/TIB2_20 at 127/138/139/146 -> class 2;
   - TIB3_01/TIB3_12/TIB3_13/TIB3_20 at 147/158/159/166 -> class 3.

3. Exhaust all 256 selectors and assert 2/3/4 versus default descriptor
   equality. Include named assertions for 0, 1, 5, and 255.
4. Assert a false-flag in-range overlay returns `None`.
5. Assert a flagged stray runtime slot 167 returns class 0, while the same
   false-flag slot returns `None`.
6. Build an overlap fixture where type 0 does not match and types 1 and 2 both
   use default aliases; slot 102 must return id 1.
7. Assert unknown overlay ID and empty tiberium registry return `None`.
8. Retain the flat-art test and prove its result contains exactly twelve
   primary variants, never extra images.

## Task 3: Migrate ore-growth consumers and repair their shared fixture

**File:** `src/sim/ore_growth.rs`

1. In `add_native_growth_queue_cell`, receive `type_id` directly, look up the
   type/class by that id, and leave density gating and the single
   `rng.next_u32()` position unchanged.
2. In `rebuild_native_tiberium_queues_from_overlays`, receive `type_id`
   directly and preserve iteration, flat-cell gating, percentage checks,
   density thresholds, source-object gating, zero priority, and class order.
3. Make local `current_tiberium_type` return the classifier result directly.
4. Leave `place_native_spread_tiberium` on
   `flat_tiberium_variant_ids`; its RNG bound and draw count remain unchanged.
5. Rebuild `tiberium_rebuild_fixture` with stock-shaped compact slots through
   TIB2_20 and include Riparius, Cruentus, and Vinifera types.
6. Preserve every existing expectation under the repaired IDs.
7. Add a production rebuild test using TIB2_20/runtime id 146, density below
   the growth ceiling, growth enabled, and spread disabled. Assert:

   - one growth entry with zero priority exists only in class 2;
   - class 0 and every other class remain empty;
   - no `ResourceNode` or RNG input supplies the result.

8. Strengthen the existing `add_native_growth_queue_cell` test:

   - clone or duplicate the seeded RNG before one accepted insert;
   - advance the expected RNG by exactly one `next_u32`;
   - compare exact `logical_state()` values, not the hashed `state()`
     fingerprint;
   - prove density rejection leaves RNG unchanged.

## Task 4: Migrate reduction and make its test non-vacuous

**File:** `src/sim/tiberium/mod.rs`

1. Make reduction-side `current_tiberium_type` return the classifier result
   directly before the overlay is cleared.
2. Rebuild `native_tiberium_fixture` with stock-shaped slots through TIB2_20
   and include at least Riparius, Cruentus, and Vinifera types.
3. Rewrite the native reseed test so the removed cell and eligible neighbors
   use TIB2_20/runtime id 146.
4. Assert full reduction:

   - clears the removed overlay and its bitmap bit;
   - inserts accepted neighbors only into native class 2;
   - leaves class 0 and every wrong class empty;
   - preserves verified neighbor order.

5. Clone the RNG immediately before reduction, reproduce exactly one raw draw
   per accepted neighbor on the expected RNG, and compare exact
   `logical_state()` values.
   Do not use a mere `assert_ne!`.
6. Leave the separately blocked density-zero/recalc behavior unchanged.

## Task 5: Validate and commit the feature

1. Format only the three owned files:

   ```text
   rustfmt --edition 2024 src/map/overlay_types.rs src/sim/ore_growth.rs src/sim/tiberium/mod.rs
   ```

2. After checking Cargo ownership, run serially:

   ```text
   cargo test -p vera20k --lib map::overlay_types::tests -- --nocapture
   cargo test -p vera20k --lib sim::ore_growth::tests -- --nocapture
   cargo test -p vera20k --lib sim::tiberium::tests -- --nocapture
   cargo check -q
   ```

3. Record every literal `test result:` line and the check exit code.
4. Run `rg` to prove no `tiberium_overlay_mapping` or
   `TiberiumOverlayMapping` remains and every type-only consumer uses the new
   API.
5. Require `git diff --check` and exactly the three owned tracked paths.
6. Commit one coherent reviewed GSI-04.03A milestone on the feature branch.

## Task 6: Guarded integration into `dev`

1. Reconcile root `dev` again: require clean status, capture its current SHA,
   verify no operation in progress, and recheck stashes, worktrees, protected
   dirty paths, agents, and Cargo ownership.
2. If `dev` advanced, reassess all touched interfaces and validation
   assumptions before proceeding. Preserve the old branch if rebase/rebuild is
   required; never rewrite another owner's work.
3. Prove the feature commit is reachable from its branch, not from `dev`, and
   contains only the three owned paths.
4. Run the same test matrix and `cargo check -q` on clean pre-merge `dev`.
5. Run `git merge --no-ff --no-commit <feature-branch>`.
6. On the combined state, rerun the same test matrix, impact search,
   `git diff --cached --check` (or `git diff HEAD --check`), and
   `cargo check -q`.
7. Commit the merge only if every combined check passes. Otherwise abort the
   no-commit merge and preserve both lines of work.
8. Append exact SHAs, changed paths, literal results, residuals, and the next
   parent-loop action to the crash-safe journal.
9. Require the feature worktree clean and merged, then remove it non-force.
   Retain the feature branch as provenance and never push.

## Task 7: Unwind and retest the available miner/ore parent seams

1. Pop GSI-04.03A from the dependency stack and re-read the parent-loop journal
   entries and current source paths.
2. Do not claim a complete level-zero acquisition fixture: current
   `miner_system.rs` still rejects `ResourceNode.remaining == 0`, and existing
   scan/archive tests use productive nodes. Retest the available adjacent
   production seams with exact existing filters:

   ```text
   cargo test -p vera20k --lib sim::miner::miner_tests::scan_ring_0_allows_harvesters_own_cell -- --nocapture
   cargo test -p vera20k --lib sim::miner::miner_tests::pick_best_resource_node_prefers_higher_density -- --nocapture
   cargo test -p vera20k --lib sim::miner::miner_tests::exit_pad_preserves_archive_on_arrival -- --nocapture
   cargo test -p vera20k --lib sim::miner::miner_tests::move_to_ore_target_stable_when_world_unchanged -- --nocapture
   ```

3. Rerun the exact classifier and queue/reduction suites from this feature.
   Rerun the production rules/overlay routing tests from GSI-04.03B, including
   both retail tests explicitly with `RA2_DIR`, `--lib`, and `--ignored`, then
   run final `cargo check -q`.
4. Classify every remaining parent dependency as integrated, still suspended,
   or bounded residual. GSI-07.15 remains suspended because the
   `remaining == 0` eligibility rejection prevents a complete parent-loop
   retest; Rust-only adjacent tests do not certify parity.
5. Record that GSI-04.04/04.06 synchronous cell recalculation, GSI-04.09
   level-zero cleanup/full effects, GSI-01.05 scheduler ordering,
   signed/reread selector behavior, and map-side new-type allocation remain
   separate unless current evidence proves otherwise.
6. Leave `dev` clean, no Cargo process owned by this task, no active child
   agent, no temporary feature worktree or copied ignored `ini/`, no remote
   push, and an exact crash-safe handoff for the user's new session.

## Post-Plan Self-Review

- The three Rust paths are disjoint from protected stash/damage/sidebar work.
- Every representable `u8` selector is covered; signed `-1` is not
  overclaimed.
- Compact-slot, flag, first-match, fallback, empty, and unknown cases have
  non-vacuous tests.
- Production rebuild and reduction tests use class 2 so fallback 0 cannot hide
  failure.
- RNG checks are attached only to paths that actually consume RNG.
- RNG checks compare `logical_state()`, not the hash fingerprint.
- Flat art remains exactly twelve images.
- No dependency, state format, snapshot, render, UI, audio, or network change
  is planned.
- No execution-time user selection remains.

## Independent Review Resolution

- The review found the linked worktree needs a physical ignored `ini/` copy
  for compile-time assets; Task 0 and cleanup now own it explicitly.
- Every focused Cargo invocation is library-scoped to avoid unrelated test
  binaries.
- The descriptor construction contract and equality derives are executable.
- Cruentus's excluded endpoint is a flagged fallback oracle.
- The GSI-04.03B contract path and parent-loop retail commands are exact.
- The parent unwind names existing adjacent miner tests and explicitly keeps
  the unavailable level-zero loop suspended.
- Combined staged validation uses a staged-aware whitespace check.
- No unresolved load-bearing review finding remains.
