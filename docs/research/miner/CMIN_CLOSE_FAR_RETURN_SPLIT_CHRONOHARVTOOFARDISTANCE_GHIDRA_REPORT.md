# CMIN Close/Far Return Split - ChronoHarvTooFarDistance Ghidra Report

**Address(es):** `0x0073E5E0`, `0x0043C2D0`, `0x00670003`, `0x004D9290`, `0x004D8FB0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard YR Chrono Miner full-cargo return split: close HELLO/refinery-radio path versus far/refused fallback `QueueingCell` staging, exact `ChronoHarvTooFarDistance` use/default, and accepted `CAN_DOCK` anchor distinction.  
**Non-Scope:** unload drain, post-empty release, two-miner handoff frame order, destroyed/sold refinery mid-unload, and exact rendered teleport pixels.  
**Confidence:** High for static split/threshold/coordinate selection; Medium for exact rendered arrival cadence because no runtime debugger frame capture was taken.  
**Active in YR:** Yes. Stock `[CMIN]` is `Harvester=yes`, `Teleporter=yes`, `Dock=NAREFN,GAREFN`; stock `[General] ChronoHarvTooFarDistance=50`; stock refineries have `DockUnload=yes`.

## 1. Overview

In `UnitClass::Mission_Harvest` state 2, a loaded Chrono Miner first tries the close refinery radio path if its object-coordinate distance to the selected refinery is within `ChronoHarvTooFarDistance`. For standard YR this is an inclusive `<= 50 * 256` lepton comparison after a 3D distance calculation.

If the close branch is not used because the distance is greater than the threshold or the close HELLO is refused, state 2 performs a fallback dock search and sends the miner to a nearby passable cell seeded from the refinery's art `QueueingCell=4,1`. That staging cell is separate from the later accepted `CAN_DOCK` cell, which is hardcoded by the refinery receiver as building anchor `+(3,1)`.

## 2. Key Fields And Constants

| Field / value | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass+0xBC` | `Mission_Harvest` substate; `2` return, `3` queue Mission Enter | `0x0073E5E0` switch | Yes |
| `UnitTypeClass+0xCD4` | `Teleporter=yes`; selects Chrono threshold branch | `0x0073E5E0`, `rulesmd.ini [CMIN] Teleporter=yes` | Yes for CMIN |
| `RulesClass+0xD7C` | `ChronoHarvTooFarDistance` | `0x0073E5E0`, `0x00670003` | Yes |
| `50 * 0x100 = 12800` | stock close threshold in leptons | `rulesmd.ini:294`, `0x0073E5E0` multiply by `0x100` | Yes |
| `BuildingTypeClass+0x1618/+0x161C` | art `QueueingCell` X/Y, fallback staging only | `0x0073E5E0`, prior ReadINI reports, `artmd.ini` | Conditional |
| hardcoded `+(3,1)` | accepted `CAN_DOCK` cell payload | `0x0043C2D0` case `0x0E` | Yes |

## 3. Core Logic

### State-2 Close Branch

`UnitClass::Mission_Harvest @ 0x0073E5E0` reads `UnitTypeClass+0xCD4` into the local teleporter flag. For non-chrono harvesters it compares against `RulesClass+0xD78` (`HarvesterTooFarDistance`). For Chrono Miners it compares against `RulesClass+0xD7C` (`ChronoHarvTooFarDistance`).

The comparison uses object coordinates from the miner and the selected refinery:

```text
dx = miner.coord.x - refinery.coord.x
dy = miner.coord.y - refinery.coord.y
dz = miner.coord.z - refinery.coord.z
distance = ftol(Sqrt_Approx(dx*dx + dy*dy + dz*dz))
close if distance <= Rules.ChronoHarvTooFarDistance * 0x100
```

The boundary is inclusive. A miner exactly 50 cells from the refinery object coordinate is still close in stock YR. Only `distance > 12800` leptons falls to the far path when the close radio path otherwise has a candidate.

If close, state 2 sends radio `0x02` (`HELLO`) to the refinery object. Only when the return value is `1` does it write substate `+0xBC = 3`. It does not send `CAN_DOCK(0x0E)` from state 2.

### Far Or Refused Fallback

When the close branch is not taken, state 2 increments `g_MapEditorMode`, calls the dock search with arg3 `1`, then decrements `g_MapEditorMode`. With a fallback refinery found, CMIN's `Teleporter=yes` makes the fallback destination branch run regardless of the local `distance > 0x300` check.

The fallback destination seed is:

```text
anchor = signed_floor_div_256(refinery.object_coord.x/y)
seed = anchor + BuildingType.QueueingCell
actual = Find_Nearby_Passable_Cell(seed, radius=2, ...)
Set_Destination(actual CellClass*) or clear destination if no valid cell
```

For stock `GAREFN` and `NAREFN`, `QueueingCell=4,1`, so a refinery anchored at `(10,10)` seeds `(14,11)`. This path does not use `NumberOfDocks`, `DockingOffset%d`, or the accepted `CAN_DOCK` anchor.

### Accepted CAN_DOCK Anchor

`BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` handles `CAN_DOCK`. In the standard `DockUnload=yes` refinery path, it calls the building cell getter and constructs:

```text
accepted = building_anchor + (3, 1)
```

The building sends this cell through radio `0x12`. Only if the harvester replies `0x14` (`ALREADY_THERE`) does the building then send `0x18` and `0x16`.

This receiver branch does not read `BuildingTypeClass+0x1618/+0x161C`. Therefore stock `QueueingCell=4,1` is a waiting/fallback staging coordinate, not the accepted dock coordinate.

## 4. INI Keys

| INI key | Stock value | Effect | Evidence | Active in YR |
|---|---:|---|---|---|
| `[General] ChronoHarvTooFarDistance` | `50` | CMIN close/far split threshold, converted by `*0x100` | `rulesmd.ini:294`, `0x0073E5E0`, `0x00670003` | Yes |
| `[General] HarvesterTooFarDistance` | `5` | non-chrono sibling threshold | `rulesmd.ini:293`, `0x0073E5E0` reads `+0xD78` for non-teleporter | Yes for HARV |
| `[CMIN] Dock` | `NAREFN,GAREFN` | candidate refinery list | `rulesmd.ini:7361` | Yes |
| `[CMIN] Harvester` | `yes` | activates harvester mission flow | `rulesmd.ini:7364` | Yes |
| `[CMIN] Teleporter` | `yes` | selects chrono threshold and fallback branch behavior | `rulesmd.ini:7396`, `UnitType+0xCD4` | Yes |
| `[GAREFN]/[NAREFN] QueueingCell` | `4,1` | far/refused fallback seed only | `artmd.ini:1773`, `artmd.ini:1716`, `0x0073E5E0` | Conditional |

## 5. Current Rust Implementation Status

Current Rust no longer matches the older "hardcoded 2-cell inbound warp threshold" warning in the prior far-return report.

Observed current Rust surfaces:

| Surface | Current behavior | Status |
|---|---|---|
| `src/rules/ruleset.rs` | parses `[General] ChronoHarvTooFarDistance`, default `50` | aligned for stock YR |
| `src/sim/miner/mod.rs` | copies `chrono_harv_too_far_distance` into `MinerConfig::too_far_threshold_chrono` | aligned for stock YR |
| `src/sim/miner/miner_system.rs::chrono_return_exceeds_too_far_threshold` | computes 3D object-coordinate lepton distance and returns true only when squared distance is greater than threshold squared | aligned for stock positive thresholds |
| `src/sim/miner/miner_system.rs::try_issue_chrono_far_return_teleport` | teleports only when the helper says the threshold is exceeded | aligned conceptually |
| `src/sim/miner/miner_system.rs::try_begin_chrono_close_return_radio` | attempts HELLO/contact path when helper says the threshold is not exceeded | aligned conceptually |
| `src/sim/miner/miner_dock_sequence.rs` | separates `refinery_queue_cell` (`QueueingCell`) from `refinery_can_dock_queue_cell` (`+(3,1)`) | aligned |

Residual implementation caveat: Rust clamps configured threshold values to at least `1` in `MinerConfig::from_general_rules`. This is not a stock-YR issue because the standard value is `50`, but a modded `ChronoHarvTooFarDistance=0` would need a separate binary check before claiming exact mod parity.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| CMIN threshold source | verified | `0x0073E5E0`, `0x00670003`, `rulesmd.ini:294` | none for stock YR |
| inclusive close comparison | verified | `0x0073E5E0` decompile: `iVar8 <= Rules+0xD7C * 0x100` | none |
| object-coordinate 3D distance | verified | `0x0073E5E0` subtracts x/y/z coords before `Sqrt_Approx` | none |
| close branch radio output | verified | `0x0073E5E0` sends `0x02` and sets `+0xBC=3` only on reply `1` | none |
| far/refused fallback `QueueingCell` staging | verified | `0x0073E5E0` reads `BuildingType+0x1618/+0x161C`, then `Find_Nearby_Passable_Cell` | exact candidate ordering delegated to prior passable-cell docs |
| accepted `CAN_DOCK` anchor | verified | `0x0043C2D0` builds anchor `+(3,1)` and sends `0x12` | none |
| current Rust threshold scan | verified from source | `src/sim/miner/miner_system.rs`, `src/sim/miner/mod.rs`, `src/rules/ruleset.rs` | modded zero/negative threshold behavior unchecked |
| unload/post-arrival runtime frames | deferred | out of scope | sibling slots |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is ChronoHarvTooFarDistance=50 active for standard YR CMIN? -> Yes; stock CMIN has Teleporter=yes, state 2 reads RulesClass+0xD7C, and rulesmd has value 50.` (evidence: `0x0073E5E0`, `0x00670003`, `rulesmd.ini:294`, `rulesmd.ini:7396`)
- `[RESOLVED] OQ-2 - Is the comparison cell-distance or lepton-distance? -> Lepton-distance. The binary computes 3D object-coordinate deltas, square-root distance, ftol, then compares to cells multiplied by 0x100.` (evidence: `0x0073E5E0`)
- `[RESOLVED] OQ-3 - Is the boundary inclusive? -> Yes. Close branch uses <=; only greater-than-threshold goes far.` (evidence: `0x0073E5E0`)
- `[RESOLVED] OQ-4 - What does close success send? -> HELLO/radio 0x02 only, then substate 3 on reply 1; no state-2 CAN_DOCK.` (evidence: `0x0073E5E0`)
- `[RESOLVED] OQ-5 - What cell does far/refused fallback target? -> art QueueingCell seed plus nearby-passable radius-2 search.` (evidence: `0x0073E5E0`, `artmd.ini:1716`, `artmd.ini:1773`)
- `[RESOLVED] OQ-6 - What cell does accepted CAN_DOCK target? -> building anchor +(3,1), not QueueingCell=4,1.` (evidence: `0x0043C2D0`)
- `[RESOLVED] OQ-7 - Is the older 2-cell Rust threshold correct? -> No for gamemd. Binary uses ChronoHarvTooFarDistance * 256, stock 50 cells. Current Rust scan no longer shows the old 2-cell threshold in this path.` (evidence: `0x0073E5E0`, source scan)
- `[DEFERRED] OQ-8 - What does stock binary do for modded ChronoHarvTooFarDistance <= 0?` (category: out-of-scope; reason: standard YR value is 50 and no modded runtime case was requested; next-step-if-pursued: breakpoint state 2 with a rules override)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Chrono close/far split uses inclusive 3D lepton distance against `ChronoHarvTooFarDistance * 0x100`; stock value is 50 cells. | `0x0073E5E0`, `0x00670003`, `rulesmd.ini:294` | current Rust appears aligned for stock values | `src/rules/ruleset.rs`; `src/sim/miner/mod.rs`; `src/sim/miner/miner_system.rs` | Keep threshold data-driven and strict-greater for far. | `chrono_return_at_exact_too_far_threshold_uses_close_radio_path`; `chrono_return_over_too_far_threshold_uses_queueingcell_teleport` | Do not reintroduce any 2-cell inbound warp threshold for full-cargo return. |
| Close success sends only HELLO and moves to the later Mission Enter path; far/refused fallback stages at `QueueingCell`. | `0x0073E5E0`, `artmd.ini:1716`, `artmd.ini:1773` | current Rust models HELLO then MissionEnter and stages refused/far miners | `src/sim/miner/miner_system.rs`; `src/sim/miner/miner_dock_sequence.rs` | Preserve one phase boundary after close HELLO and use `QueueingCell` only when close path is not accepted or distance is over threshold. | `cmin_close_hello_success_defers_can_dock_to_mission_enter`; `chrono_close_hello_refused_stages_at_queueingcell_without_receiver_eviction` | Do not collapse HELLO, CAN_DOCK, and accepted-cell movement into the same state-2 operation. |
| Accepted `CAN_DOCK` cell is hardcoded anchor `+(3,1)`, while art `QueueingCell=4,1` is fallback/wait staging. | `0x0043C2D0`, `0x0073E5E0` | current Rust has separate helpers | `src/sim/miner/miner_dock_sequence.rs`; `src/sim/miner/miner_tests.rs` | Keep `refinery_queue_cell` and `refinery_can_dock_queue_cell` distinct. | `queued_miner_uses_queueingcell_only_until_contact_then_can_dock_anchor_cell` | Do not use art `QueueingCell` as the stock accepted dock target. |

## 9. Negative Facts / Do Not Do

- Do not use a 2-cell threshold for standard CMIN full-cargo return. The binary uses `ChronoHarvTooFarDistance=50` cells converted to `12800` leptons.
- Do not measure ore-field-to-refinery distance for this state-2 return split. The verified comparison is miner object coordinate to candidate refinery object coordinate.
- Do not send `CAN_DOCK(0x0E)` from `Mission_Harvest` state 2 close success. State 2 sends only `HELLO(0x02)` and writes substate 3 on reply `1`.
- Do not use art `QueueingCell=4,1` as the accepted `CAN_DOCK` target. Accepted refinery admission uses anchor `+(3,1)`.
- Do not use `NumberOfDocks` or `DockingOffset%d` for the far/refused fallback staging coordinate.

## 10. Remaining Uncertainty

- Exact rendered teleport arrival cadence and first-pixel movement are runtime-visual questions outside this slice.
- Modded zero/negative `ChronoHarvTooFarDistance` behavior was not checked; current Rust clamps to at least `1`, which is safe for stock YR but not proven for arbitrary rules overrides.

## 11. Stale Docs / Follow-up Wording

- `CHRONO_MINER_FAR_RETURN_FALLBACK_DESTINATION_GHIDRA_REPORT.md` section 6 — **[RESOLVED 2026-05-25, re-confirmed 2026-07-19]** this pointer is itself now stale: that doc's §6 (and its top supersession note) were already updated on 2026-05-25 and no longer claim a hardcoded 2-cell inbound warp threshold. Its §5 also already attributes the inbound warp to the teleport *locomotor* (StateMachineTick), not to state-2 — consistent with the phase-3 chrono-warp-trigger reframe. No edit needed there.
- `docs/research/units/allied/CMIN.md` wording that describes the split as ore-field distance should be narrowed. Replacement wording: "In `Mission_Harvest` state 2, CMIN compares its current object-coordinate distance to the candidate refinery against `ChronoHarvTooFarDistance * 256`; ore-field distance is not the measured value for this return split."

## Sources

- Ghidra read-only decompile: `UnitClass::Mission_Harvest @ 0x0073E5E0`.
- Ghidra read-only decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`.
- Ghidra read-only decompile: `RulesClass::ReadGeneral @ 0x00670003`.
- Existing reports: `docs/research/miner/CMIN_STATE2_CLOSE_FAR_RETURN_TO_MISSION_ENTER_DISPATCH_GHIDRA_REPORT.md`; `docs/research/miner/CHRONO_MINER_FAR_RETURN_FALLBACK_DESTINATION_GHIDRA_REPORT.md`; `docs/research/miner/RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md`; `docs/research/UNIT_MISSION_ENTER_REFINERY_RETRY_QUEUE_LOOP_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`; `ini/rules.ini`; `ini/artmd.ini`; `ini/art.ini`.
- Rust scan: `src/rules/ruleset.rs`; `src/sim/miner/mod.rs`; `src/sim/miner/miner_system.rs`; `src/sim/miner/miner_dock_sequence.rs`; `src/sim/miner/miner_tests.rs`.

## Status

COMPLETE
