# P4/P5 per-step charge authority — validation handoff (2026-07-27)

## Outcome

The goal's code was already landed on `dev` on 2026-06-06 by prior sessions; this session
verified it live and ran the validation. No code changes were made, no new
SNAPSHOT_VERSION bump was needed (P5's bumps 17→18→19 are history; current value is 29
from later slices).

Commits of record (all on `dev`):
- `a1e1daeb` P4 — FIFO queue + cancel-first-match + partial refund (oracle)
- `dc7a34d9` P5a — build-step-time producer + category routing
- `1db41ebf` P5b — THE AUTHORITY FLIP (registry authoritative, per-step charge to real
  wallet, SNAPSHOT_VERSION 17→18)
- `5770151d` P5c — acceptance gate (A determinism + B conservation)
- `06eae652` P5d — queue-of-record in `Factory.queue` (18→19)

## Validation

- Live-code check: enqueue has **no upfront debit** (`production_queue.rs:187,197` —
  per-step `advance_one_step` owns the cost); cancel of the **active** build refunds
  exactly the spent portion `original_balance - balance` (C8, `factory.rs::cancel_active`);
  a queued (tail) item was never charged and refunds **nothing**; first-front-to-back
  match order (legacy `.rev()` last-match retired).
- Full suite this session: `cargo test -p vera20k --lib` → `test result: ok. 4981 passed;
  0 failed`.
- In-game (2026-07-27 ~15:14–15:18, quickplay, local player 'Americans', 10,000
  credits): user-driven match with command timeline verified from `logs/ra2.log` —
  MCV deploy, GAPOWR queued 15:14:36 and canceled mid-bar 15:14:42, GAPOWR + GAPILE
  built and placed, GAREFN queued 15:15:09 and canceled 15:18:22. User visually
  confirmed the credit behavior: "it looks good to me". Identity pinning confirmed
  (commands issue as the pinned local house).

## Residuals

1. **Stall-at-zero-credits visual check is coarse**: the validation match confirmed
   cancel refunds visually and by command timeline, but the wallet was not provably
   driven to 0 mid-bar in-game (10k start; user reported overall behavior looks good).
   The stall/resume path is covered by the P5c conservation/determinism gate tests.
2. **Sidebar shows unbuildable cameos**: with zero strict-buildable items (pre-deploy),
   the sidebar still showed a POWER PLANT cameo; gamemd hides missing-prereq items.
   Player-visible at every match start until the MCV deploys. Out of this goal's scope.
3. **BUILD-DIAG log spam**: the zero-options diagnostic in
   `production_queue.rs::build_options_for_owner` fires up to 3×/tick per owner at WARN
   (90k+ lines in `logs/ra2.log`, which also spans months — consider rotation). Demote or
   remove.
4. IncomeMult is NOT a residual — it landed in P7 (`c220e2d0`).

## Repo state / next safe action

This session changed no tracked files. The dirty tree (rmg/loading-screen files) and the
PENDING_REBASELINES entries belong to other sessions — untouched. Next safe action: user
plays a match (deploy → Power Plant → cancel at half-bar → build until broke) and
confirms the two behaviors; if either misbehaves, start from `factory.rs::cancel_active`
and `advance_one_step`.
