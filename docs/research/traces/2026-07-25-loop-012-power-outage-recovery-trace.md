# LOOP-012 Power Outage and Recovery Trace

Date: 2026-07-25  
Loop: `LOOP-012-POWER-OUTAGE-RECOVERY`  
Owner: `GSI-09.07`  
Fixture: stock American Allied house with `GACNST`, one active `GAPOWR`, one active
`AMRADR`, sufficient credits, no blackout timer, no EMP/warping state, no
`FreeRadar`, and no SpySat override.

## Verdict

**FAIL at stage 2 (`GSI-09.14`, sell/teardown).**

The current Rust command path removes a sold `GAPOWR` and credits its refund
immediately. Active `gamemd.exe` instead advances a three-state building sell
mission, keeps the power plant live and power-producing while its reverse
buildup animation runs, and performs the refund plus `UnInit` only after the
animation-completion byte is set. The outage, radar loss, sidebar transition,
and low-power EVA therefore begin too early in Rust.

This is the earliest load-bearing divergence in the selected loop. The parent
`GSI-09.07` work is suspended until the smallest `GSI-09.14` prerequisite slice
can provide durable, ordered sell-mission state.

## Evidence levels

- **VERIFIED-BINARY**: read directly from the active retail
  `gamemd.exe` loaded in Ghidra, with the concrete function/address named below.
- **RETAIL-DERIVED**: read from the repository's stock RA2/YR INI overlay or
  configured retail assets.
- **RUST-SOURCE-CONFIRMED**: read from current committed Rust source at
  merged `dev` commit `84991f1404cd804f1ea8252afe4da3e0943898bd`.
- **UNCHECKED**: not yet proven by a native executable oracle or exhaustive proof.

Rust regression tests are not native parity proof.

## Stock fixture values

YR `rulesmd.ini` patches base `rules.ini`.

| Item | Stock value | Evidence |
|---|---:|---|
| `GAPOWR.Cost` | 800 | RETAIL-DERIVED |
| `GAPOWR.Strength` | 750 | RETAIL-DERIVED |
| `GAPOWR.Power` | +200 | RETAIL-DERIVED |
| `GAPOWR.Buildup` | `GAPOWRMK` | RETAIL-DERIVED |
| `GAPOWR.Crewed` | yes | RETAIL-DERIVED |
| `AMRADR.Power` | -50 | RETAIL-DERIVED |
| `AMRADR.Radar` | yes | RETAIL-DERIVED |
| `[General] RefundPercent` | 50% | RETAIL-DERIVED |
| `[IQ] SellBack` | 2 | RETAIL-DERIVED; AI permission threshold, not refund percentage |

The initial house total is output 200, drain 50. `GACNST` is retained so the
house can actually queue and place the replacement power plant through the
production path.

## Mechanism blocks

### 1. Player sell command and mission authority

**Native authority**

`BuildingClass::Mission_Selling` at `0x00449C30` is a three-state mission:

1. State 0 initializes selling, clears the animation-completion byte, broadcasts
   the sell-start radio event, clears conflicting animations, and advances to
   state 1.
2. State 1 ejects occupants/crew as applicable, broadcasts the next radio event,
   plays the sell sound, advances to state 2, calls
   `BuildingClass::GrandOpening(0)`, and clears animation completion.
3. State 2 waits for the completion byte at building offset `+0x6DD`. Only then
   does it dirty the owner's derived state, calculate/credit the refund, perform
   ore-purifier consequences when applicable, and call `UnInit`.

Evidence: live Ghidra decompile and disassembly of `0x00449C30`; cross-check with
`BuildingClass::GrandOpening` at `0x00447780` and
`BuildingClass::UpdateAnimation` at `0x004509D0`.

**State reads/writes**

- Reads and advances the building mission substate.
- Writes building-state/animation fields through `GrandOpening(0)`.
- Clears and later waits on animation completion at `+0x6DD`.
- Does not mark the building dying or remove it at the command edge.
- Dirties house power/radar-derived state only in completion state 2.

**Timing**

`GrandOpening(0)` selects building state 0 and loads its frame/timer entry from
the BuildingType building-state table. `UpdateAnimation` advances by the stored
frame step when the timer expires, and sets `+0x6DD` after reaching
`start + count - 1`. The duration is asset/type-table driven, not a fixed
simulation-duration approximation.

`BuildingTypeClass::LoadVisualAssets` at `0x0045F230` reads the raw SHP frame
count, divides it by two to exclude the shadow half, stores `start = 0` and that
body-frame count, and computes the timer duration as
`round(Rules.BuildupTime * 900 / count)`. `Math__ftol` at `0x007C5F00` supplies
the rounding operation. The stock rules set `BuildupTime=.06`. All six retail
power-plant make variants (`GDPOWRMK`, `GNPOWRMK`, `GLPOWRMK`, `GUPOWRMK`,
`GTPOWRMK`, and `GAPOWRMK`) contain 50 raw SHP frames, or 25 body frames.
Consequently the stock state entry is `start=0`, `count=25`, and timer
duration `round(.06 * 900 / 25)=2` logic frames. The completion byte is written
when the current body frame reaches 24, after 24 two-frame timer expiries from
the frame-0 seed (48 animation-clock increments). Whether the selling mission
observes that byte in the same scheduler pass or the next remains **UNCHECKED**.

Evidence: live Ghidra decompile/disassembly of `0x0045F230`,
`0x00447780`, `0x004509D0`, and `0x007C5F00`; stock
`rules.ini`/`rulesmd.ini`; and read-only extraction of the retail theater MIX
archives and SHP headers.

**Current Rust**

`src/sim/production/production_sell.rs::sell_building` ejects occupants, calls
`uninit_entity`, applies vision/superweapon consequences, computes a refund,
and credits it during the sell command itself.

Result: **DRIFT**. The plant disappears before the native reverse buildup would
finish, so the power loop is entered early.

### 2. Sell refund

**Native authority**

The sell completion wrapper at `0x0070ADA0` obtains the object TechnoType and
calls its virtual cost/refund method. The BuildingType implementation at
`0x00711F60` loads the Rules singleton and reads the double at Rules
`+0x1738`, populated by `[General] RefundPercent`. The explicit boolean passed
by building selling is zero, so the configured multiplier remains active.
Owner cost bonuses participate before x87 integer conversion.

Evidence: live Ghidra disassembly at `0x0070ADA0` and
`0x00711F60`; `RulesClass::ReadIQ` at `0x00674240` separately proves Rules
`+0x145C` is `[IQ] SellBack`, the AI sell-permission threshold.

For the full-health, unmodified stock fixture, the expected base refund is
`800 * 0.50 = 400` credits. The general formula must not be hardcoded and must
not be health-scaled merely because the building is damaged.

**Current Rust**

`SELL_REFUND_PERCENT` is hardcoded to 50 and
`sell_refund_for_building` scales the result by current health.

Result: **DRIFT** in mechanism and non-fixture cases. The stock full-health
amount happens to be 400, but it is credited at the wrong time.

### 3. Power reassessment

**Native authority**

`HouseClass::Update` at `0x004F8440` handles blackout timers, then calls
`HouseClass::AI_AssessPower` at `0x00508C30` when power is dirty, followed by
radar reassessment in the same house update.

`AI_AssessPower`:

- records the old low-power predicate as `output < drain && drain != 0`;
- clears dirty state and totals;
- iterates the house's owned, live, not-being-destroyed buildings in native
  house-list order;
- adds health-scaled production and full drain;
- handles blackout and garrison/reactor contributions;
- recalculates factory production consequences before publishing the transition;
- marks radar for recheck.

A building in buildup/selling animation is not excluded merely for being in
that visual state. It remains power authority until lifecycle teardown.

**Current Rust**

`src/sim/power_system.rs::recalculate_power_for_owner` iterates deterministic
entity order and skips dying entities. It no longer treats the presentation-only
`building_up` state as lifecycle authority: every live owned structure reaches
the existing health-scaled output, full-rated drain, and reactor-occupant
formula. The world still does not consume every returned transition directly.

Results:

- **DRIFT caused upstream**: selling removes `GAPOWR` too early.
- **CLOSED for the bounded replacement slice**: a newly placed full-health stock
  `GAPOWR` contributes 200 output while its visible buildup remains active.
- The exact native x87 rounding equivalence of Rust's integer health scaling is
  **UNCHECKED**.

### 4. Radar gate

**Native authority**

`HouseClass::UpdateTacticalRadarAvailability` at `0x00508DF0` is local-house
only, honors `FreeRadar`, compares current output and drain, and evaluates the
first eligible `Radar=yes` building candidate. EMP/warping state gates that
candidate. The search stops after the first radar candidate whether it succeeds
or fails. SpySat/shroud authority is separate.

For the fixture's only `AMRADR`:

- before sell completion: 200 output / 50 drain -> radar remains enabled;
- after plant teardown: 0 output / 50 drain -> radar is disabled;
- after the replacement plant becomes live power authority: 200 / 50 -> radar
  is enabled again.

**Current Rust**

`src/sim/power_system.rs::has_active_radar` now applies the aggregate house
low-power gate and then searches live owned `Radar=yes` structures without a
`building_up` exclusion. It no longer conflates the separate `SpySat=yes`
full-map reveal authority with tactical radar. Rust still uses an any-match
search rather than native first-candidate termination, and does not model the
candidate EMP/warping gate or `FreeRadar`.

Result for this loop: the bounded placement/recovery gate is closed; the sell
lifecycle and the explicit radar residuals remain **DRIFT/UNCHECKED**.

### 5. Sidebar, minimap, and audio handoffs

**Native ordering**

`HouseClass::Update` performs power assessment before radar reassessment and
then evaluates the local low-power notification guard. Full power resets that
guard. A deficit with an owned configured power-plant type and a clear guard
plays the low-power EVA/message once and sets the guard. There is no symmetric
generic recovery EVA.

Evidence: live Ghidra decompile of `HouseClass::Update` at `0x004F8440`.

**Current Rust production consumers**

- The app polls the local `PowerState` edge in
  `src/app_sim_tick.rs::announce_local_state_evas` and queues
  `EVA_LowPower`.
- `src/sim/radar.rs` consumes `has_active_radar`.
- `src/render/radar_anim.rs` gates the minimap on the radar state.
- `src/sidebar/power_bar_anim.rs` consumes power-state changes.

No recovery EVA is currently required. Exact native presentation-frame
ordering, EVA queue priority/reannouncement, power-bar animation cadence, and
radar animation pixels are **UNCHECKED** for this pilot.

## Ordered stock loop trace

| Stage | Native expectation | Current Rust | Status |
|---:|---|---|---|
| 1 | Player issues sell for `GAPOWR` | Sell command reaches production path | PASS to entry |
| 2 | Enter three-state sell mission; building stays live | Immediate occupant ejection, refund, `UnInit` | **FAIL — earliest** |
| 3 | Refund only at animation completion; stock amount 400 | 400 at command edge for full-health stock fixture | FAIL timing |
| 4 | Teardown then marks house-derived state dirty | Teardown occurs at command edge | FAIL timing |
| 5 | Reassess to 0 output / 50 drain after teardown | Reassesses during the command tick | FAIL timing |
| 6 | Disable local radar after power assessment | Disables from early low-power state | FAIL timing |
| 7 | Power bar enters low-power presentation | Enters early; exact cadence unchecked | FAIL/UNCHECKED |
| 8 | Minimap follows radar loss | Follows early loss; exact pixels unchecked | FAIL/UNCHECKED |
| 9 | Play low-power EVA once; no recovery EVA | App edge queues low-power EVA early | FAIL timing |
| 10-14 | Select, queue, build, and pay for stock `GAPOWR` | Production pipeline exists | UNCHECKED end-to-end |
| 15-18 | Place/reveal/activate replacement through real lifecycle | Placement spawns full health with fixed 30-tick `building_up` | PARTIAL |
| 19 | Newly live plant contributes according to native lifecycle | Full-health stock `GAPOWR` contributes 200 during `building_up` | PASS for bounded mechanism |
| 20-22 | Radar, power bar, and minimap recover; no recovery EVA | House power and `Radar=yes` recover by the next guaranteed assessment while buildup remains active | PASS mechanism / UNCHECKED exact frame and pixels |

## Failure branches and neighboring assumptions

- Selling abort/failure before mission state is accepted: **UNCHECKED**.
- Occupant/crew ejection order and failed placement of survivors: outside the
  minimal power prerequisite but must remain lifecycle-correct.
- Building destroyed while selling: **UNCHECKED**.
- Player cancels/changes mission during selling: **UNCHECKED**.
- Blackout timer, EMP, warping, `FreeRadar`, multiple radar providers, and SpySat:
  not triggered by the stock fixture; broader radar/SpySat details remain
  explicit residuals.
- Production pause/slowdown under low power is a downstream consumer and must be
  revisited after the lifecycle correction.
- Exact placement-to-house-update tick depends on scheduler ordering and remains
  **UNCHECKED**.

## Production Rust touchpoints

Earliest prerequisite:

- `src/sim/production/production_sell.rs`
- `src/sim/production/production_types.rs`
- `src/sim/game_entity.rs`
- `src/sim/world/mod.rs`
- `src/sim/world/world_hash.rs`
- `src/sim/snapshot.rs`
- `src/app_instances/shp.rs`

Parent-loop resume:

- `src/sim/power_system.rs`
- `src/sim/production/production_placement.rs`
- `src/sim/radar.rs`
- `src/app_sim_tick.rs`
- `src/render/radar_anim.rs`
- `src/sidebar/power_bar_anim.rs`

The bounded power/radar implementation touched none of the durable
mission/hash/snapshot or sell-lifecycle files. The upstream sell divergence
therefore remains available as a separate future owner rather than being
silently approximated here.

## Approval question

**Why should this be approved?**

The trace follows the selected stock loop from a real sell command through
native lifecycle, power authority, radar, presentation consumers, replacement
production, placement, and recovery. It identifies the first divergence by
ordered stage rather than choosing the easiest power helper to patch. The
load-bearing native claims are grounded in current live-binary bodies and stock
INI values.

**What evidence could still make it wrong?**

- A native executable trace could expose a scheduler-frame boundary different
  from the static call-order interpretation.
- Decoding the actual `GAPOWRMK` BuildingType state table could change the exact
  completion tick.
- The other worktree may introduce a compatible lifecycle abstraction that
  changes the correct Rust touchpoints.
- Native audio queue arbitration or radar presentation timing could add a
  downstream failure not visible in this static mechanism trace.

None of those possibilities changes the proven fact that immediate Rust
teardown precedes native animation-completion teardown.

## 2026-07-25 bounded implementation update

The replacement-plant prerequisite was merged without changing the still-open
sell mission:

- feature commit `21a757c99e96d47c2930e26b5fa228d057ad6309`;
- repaired feature commit `2323e2f5cd766a55e18e711a17d0d79f68162c1f`;
- current-dev feature merge `1797cd834002af2563353ce6328f82146030000a`;
- merged `dev` commit `84991f1404cd804f1ea8252afe4da3e0943898bd`.

Additional verified binary evidence closes the factory-health ambiguity.
`BuildingClass::Init_Managers` at `0x00442C40` copies
`BuildingType.Strength` into current and displayed health before construction
returns. A stock factory-created `GAPOWR` therefore reaches placement at
750/750 health and `GetPowerOutput` yields 200, not 68. `Unlimbo` marks house
power and radar state dirty; exact command-frame House update ordering remains
**UNCHECKED**, so the production oracle observes the next guaranteed
reassessment while buildup is still active.

Literal merged-path validation:

- power tests: `21 passed; 0 failed`;
- stock placement/recovery production test: `1 passed; 0 failed`;
- radar tests: `11 passed; 0 failed`;
- `cargo check -q`: exit 0, warnings only;
- full library with only the independently reproduced pre-existing global
  replay-baseline assertion skipped:
  `4755 passed; 0 failed; 19 ignored; 1 filtered out`.

The unskipped full library run remains red only at
`global_skirmish_replay_is_deterministic_and_baseline_stable`:
`4755 passed; 1 failed; 19 ignored`, final hash
`B86BAFD0F6AAACE0`. Clean pre-merge `dev` produced the identical assertion and
hash, so it was neither caused nor rebaselined by this slice.
