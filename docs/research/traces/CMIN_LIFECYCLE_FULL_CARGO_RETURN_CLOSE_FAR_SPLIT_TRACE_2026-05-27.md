# CMIN Lifecycle Full Cargo Return Close/Far Split Trace - 2026-05-27

## Scope

Concrete scenario: a stock YR Chrono Miner (`CMIN`) has full cargo after mining and one valid same-owner stock refinery exists. Trace only the standard return selector around `ChronoHarvTooFarDistance`: a close case exactly at the threshold and a far case just over it. Stop at the selector and first visible movement/warp outcome; unload internals are out of scope.

No Rust, INI, or published research docs were modified for this trace. Ghidra MCP was not used directly; all gamemd references below come from existing verified Ghidra reports and stock INI files.

## Evidence Base

- Active YR data: stock `[CMIN]` has `Dock=NAREFN,GAREFN`, `Harvester=yes`, and `Teleporter=yes` in `ini/rulesmd.ini:7361`, `ini/rulesmd.ini:7364`, `ini/rulesmd.ini:7396`.
- Active YR refinery data: stock `[GAREFN]` and `[NAREFN]` have `DockUnload=yes` and `Refinery=yes` in `ini/rulesmd.ini:11726-11727` and `ini/rulesmd.ini:12519-12520`.
- Active YR threshold: stock `[General] ChronoHarvTooFarDistance=50` in `ini/rulesmd.ini:294`.
- Active YR far staging data: stock `QueueingCell=4,1` for `NAREFN` and `GAREFN` in `ini/artmd.ini:1716` and `ini/artmd.ini:1773`.
- Verified gamemd selector: `docs/research/miner/CMIN_CLOSE_FAR_RETURN_SPLIT_CHRONOHARVTOOFARDISTANCE_GHIDRA_REPORT.md` states the close/far comparison is 3D object-coordinate distance, inclusive `distance <= ChronoHarvTooFarDistance * 256`, with stock threshold `50 * 256 = 12800` leptons; the branch is active in standard YR.
- Verified gamemd phase boundary: `docs/research/miner/CMIN_STATE2_CLOSE_FAR_RETURN_TO_MISSION_ENTER_DISPATCH_GHIDRA_REPORT.md` states close state 2 sends only `HELLO`, writes the harvest substate to queue enter later, and reaches the harvest mission timer epilogue before state 3 can queue Mission Enter.

## Rust Touchpoints

- Threshold parsing: `src/rules/ruleset.rs:1013-1014` reads `HarvesterTooFarDistance` and `ChronoHarvTooFarDistance`; `src/sim/miner/mod.rs:231-236` copies the chrono value into `MinerConfig::too_far_threshold_chrono`.
- Distance selector: `src/sim/miner/miner_system.rs:37-68` compares object-coordinate lepton squared distance with a strict `>` threshold, so exact threshold remains close.
- Return dispatch order: `src/sim/miner/miner_system.rs:970-1028` tries the chrono far teleport when the threshold is exceeded; `src/sim/miner/miner_system.rs:907-968` handles the close radio path when it is not.
- Staging/accepted cell distinction: `src/sim/miner/miner_dock_sequence.rs:169-188` uses `QueueingCell` for waiting/far staging and hardcodes accepted `CAN_DOCK` as anchor `+(3,1)`.

## Concrete Values

Use a stock refinery anchored at `(10,10)` and axis-aligned CMIN positions so the threshold boundary is literal:

- Close boundary case: CMIN at `(60,10)` gives `dx = 50 * 256 = 12800`, `dy = 0`, `dz = 0`. Gamemd close predicate is `12800 <= 12800`, so close. Rust computes `distance_sq = 12800^2` and `threshold_sq = 12800^2`; strict `>` is false, so close.
- Far just-over case: CMIN at `(61,10)` gives `dx = 51 * 256 = 13056`, `dy = 0`, `dz = 0`. Gamemd close predicate is `13056 <= 12800`, false, so far. Rust computes `13056^2 > 12800^2`, true, so far.
- Far staging target: refinery anchor `(10,10)` plus stock `QueueingCell=(4,1)` seeds `(14,11)`. Current Rust uses the same seed and passable-cell helper in `src/sim/miner/miner_system.rs:1224-1257`.
- Accepted close `CAN_DOCK` target, when Mission Enter later runs, is `(13,11)` from anchor `+(3,1)`, not `QueueingCell=(14,11)`.

## Stage Verdicts

| Stage | Gamemd expected | Current Rust observed | Verdict |
|---|---|---|---|
| Active stock data | CMIN is a teleporter harvester, stock refineries are dock-unload refineries, threshold is 50 cells | Rules parser and miner config expose the same stock values | PASS |
| Close boundary selector | Exactly 50 cells / 12800 leptons is close because compare is inclusive | `distance_sq > threshold_sq` is false at equality | PASS |
| Far over-threshold selector | 51 cells / 13056 leptons is far | `distance_sq > threshold_sq` is true | PASS |
| Close immediate outcome | State 2 sends HELLO only; no chrono warp and no state-2 CAN_DOCK move | First tick sets dock contact/MissionEnter, no teleport state, no movement target | PASS for first visible outcome |
| Far staging target | Far/refused path targets QueueingCell passable staging, `(14,11)` in this stock setup | Focused test asserts teleport target `(14,11)` | PASS |
| Far rendered warp cadence/pixels | Requires runtime frame/pixel capture beyond existing static report | Rust creates teleport state/effects, but exact gamemd rendered cadence was not computed | UNCHECKED |
| Close first accepted-cell movement timing | Close state 2 must return through `[Harvest] Rate=.016`, giving `ftol(0.016 * 900) + RandomRanged(0,2) = 14..16` frames before state 3 queues Mission Enter; first Mission Enter dispatch follows after the queued mission is promoted | Current Rust issues the accepted-cell move on the next miner tick after HELLO acceptance (`src/sim/miner/miner_dock_sequence.rs:860-867`; test at `src/sim/miner/miner_tests.rs:1341-1353`) | FAIL |
| General non-axis Sqrt_Approx boundary shape | Gamemd uses `Sqrt_Approx` plus `ftol`; exact non-axis near-boundary behavior can differ from a squared-distance compare | Rust uses squared distance; this trace only checked axis-aligned 50/51-cell cases | UNCHECKED |

## Player-Visible Findings

1. FAIL - Close-return first movement can start too early. In stock YR, a close full CMIN return that gets HELLO accepted waits the harvest mission timer path before Mission Enter can issue the accepted-cell move; current Rust moves on the next miner tick. This can make close refinery returns visibly snappier and changes timer/RNG consumption.

No NOT-IMPLEMENTED finding was found in this scoped selector slice.

## Verification Run

Ran:

```text
cargo test -q chrono_return_
```

Result: 5 tests passed. The filter includes the exact-threshold close test, over-threshold QueueingCell teleport test, within-threshold close path test, and refused-close staging tests.

## Verdict Tally

PASS: 5 | FAIL: 1 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Adjacent Findings

- Modded `ChronoHarvTooFarDistance <= 0` remains outside this standard YR trace. Current Rust clamps threshold cells to at least 1 in `src/sim/miner/mod.rs:231-236`; the stock value is 50.
- Unload, deposit, release, and post-release return-to-ore behavior are intentionally left to sibling lifecycle slots.

## Status

COMPLETE
