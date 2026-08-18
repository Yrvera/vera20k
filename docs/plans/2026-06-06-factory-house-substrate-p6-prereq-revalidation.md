---
title: Factory/House Economy Substrate — P6 (prerequisite/factory-loss revalidation) — DONE
date: 2026-06-06
status: implemented + committed (079b0170); full suite green. Produced by the p6-prereq-revalidation
        workflow (6 Ghidra/INI/live-tree understand lanes + synthesis + adversarial review). The synthesizer
        caught that 5/6 lanes mis-identified the temp/permanent discriminator and re-verified the real gate;
        the adversarial review then caught a BLOCKER in the synthesizer's own power-suspend model. Both
        corrections are reflected in the shipped (2-way) implementation.
branch: factory-house-substrate-p1p2
---

# P6 — prerequisite / factory-loss revalidation

## What shipped (the 2-way)
Every tick, before the Phase-7 charge sweep, re-validate each active + queued build. A build whose
**prerequisites or producing factory were lost** is disposed of:
- **active in-progress build** → `cancel_active` (C8 PARTIAL refund = `original_balance - balance`, into
  `house.credits` via the Economy shim) → promote the first surviving queued entry (C7 StartNextQueued,
  cost-seeded, `step_delay=1`).
- **queued tail** → back-to-front drop (no refund — never charged).
- idle factory pruned.

Previously prereqs were checked ONLY at enqueue and never re-checked, so a build whose War Factory / tech
building was destroyed mid-progress kept charging to completion — a real, currently-missing parity gap.

## THE SCOPE CORRECTION (binary-verified) — no power-suspend
The adversarial review's **blocker**: the synthesizer modelled `TemporarilyBlocked` as "house in low-power →
suspend the build." That is WRONG against gamemd:
- gamemd's building `HasPower` (+0x198) is toggled ONLY by event-driven GoOnline/GoOffline (EMP / spy / sell
  / trigger) — `decompile 0x00452260/0x00452360`. A plain power **deficit** does NOT call GoOffline; it
  **SLOWS** production via Min/MaxLowPowerProductionSpeed (already modeled as `power_ratio_ppm` in
  `prepare_step_inputs`). gamemd shows continuous slow progress on a deficit, never a halt.
- The `(1,1,1)` gate (require-powered-factory) fails only when a factory building's HasPower is
  event-toggled down (EMP etc.), which the Rust engine does NOT model per-building yet.
- ⇒ `BuildEligibility::TemporarilyBlocked` (power-suspend) + the C9 resume are **UNREACHABLE today** and left
  as a forward seam for the EMP slice. Implementing a low-power suspend would be a player-visible BUG.

So P6 = the **2-way** (PermanentlyBlocked → abandon/drop; else Buildable). That is the reachable gamemd
behavior in the current engine.

## The gamemd gate (binary-verified)
- The disposition gate is `FindFactoryBuilding` = `FUN_005f7900` (TechnoTYPE vtable+0x94), NOT
  `HouseClass::CanBuild` directly. The two calls in `0x00509140` differ only in p3 (require-power): p3=0 =
  "any candidate factory building CanBuild-passes" ; p3=1 = "...AND is powered."
  - `(1,0,1)` fails → no candidate building satisfies CanBuild → **PermanentlyBlocked → AbandonProduction +
    StartNextQueued** (active abandon has NO IsManual guard — a paused build is abandoned too).
  - `(1,0,1)` pass, `(1,1,1)` fail → a CanBuild-passing factory exists but none powered → TemporarilyBlocked
    (the unreachable arm above).
- Refund = `Add_Credits(GetCost - Balance)` (`decompile 0x004C9FF0`) = the already-charged portion; Rust
  `cancel_active` uses `original_balance - balance` (equal for static YR costs).
- Queued tail walked **back-to-front** (`for i=Count-1..0`), failures removed, no StartNextQueued, no suspend.
- Cadence: event-driven (5 callers of `0x00509140`: Limbo/Unlimbo/GoOnline/GoOffline/ReadFromINI). The Rust
  per-tick sweep is an observably-identical pure-function-of-hashed-state equivalent (the building set is
  frozen by the production phase) and more determinism-safe (no dirty-flag wiring into co-edited event sites).
- Live in YR for humans (only the sidebar-flash sub-arm is `g_PlayerPtr`-gated).

## Files
- `src/sim/production/production_tech.rs` — `revalidate_eligibility` (reason → Buildable/PermanentlyBlocked;
  InsufficientCredits + AtBuildLimit map to Buildable).
- `src/sim/production/factory.rs` — `RevalAction` + `plan_revalidation` (read) + `apply_revalidation` (write).
- `src/sim/world/mod.rs` — one-line hook before `prepare_step_inputs` in the Phase-7 block.
- tests: `production_replay_tests.rs` (3 new) + `production_shadow_tests.rs` (vehicle_rules TechLevel=1 +
  GAWEAP factory; `spawn_war_factory` in the 3 charge-machinery guards — a build can't exist without a factory).

## Hash: NO bump. Only already-hashed `Factory` flags / `house.credits` change value (no new serialized field).

## DRIFT / deferred
- **TemporarilyBlocked power-suspend + C9 resume** — forward seam, wired when per-building EMP/power-down lands.
- **NoFactory edge** (review HIGH): Rust `has_factory_for_owner` (building present + Factory= key) ≠ gamemd
  `(1,0,1)`-NULL (no candidate passes CanBuild). They differ for a present-but-CanBuild-failing factory
  (e.g. captured/wrong-owner). Documented DRIFT; the common case (factory destroyed) matches.
- **Refund** uses the `original_balance` snapshot vs gamemd's live `GetCost` — equal for static YR costs.
- Stale docs to patch (right outcome, wrong gate label): FACTORY_CLASS_BUILD_SPEED, BUILDINGCLASS_ON_DESTROYED §3q,
  FACTORY_HOUSE...SERVICE_STUDY §5 C19 (they call the pivot CanBuild(1,0,1)/(1,1,1); it is FindFactoryBuilding
  vtable+0x94 = 0x005F7900 with the HasPower arg).
