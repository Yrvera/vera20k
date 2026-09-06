# Phase 7 closure record — Harvesting and economy (2026-09-05/06)

Rows 153–165 of
[`2026-07-30-clean-slate-system-implementation-order.md`](../../plans/2026-07-30-clean-slate-system-implementation-order.md):
GSI-09.01, 09.02, 09.03, 09.05, 07.15, 07.17, 07.37, 07.38, 07.39, 07.21,
09.04, 15.06, 15.07.

Method: nine read-only disparity scans enumerated the rows' mechanisms from
the binary (`scan-*.md`); each mechanism marked DRIFT/MISSING was then built
by one agent and reviewed by an independent read-only critic against the
gamemd.exe bodies until it passed; two reverse audits re-probed the binary
for omissions and regressions (`reverse-audit-phase7.md`, then
`reverse-audit-phase7-final.md`, whose Part C is the consolidated residual
list). Rows are recorded in the System Map as `native_evidence: ANCHORED`;
`parity` stays `UNCHECKED`/`DRIFT` because no gamemd-derived executable
comparison was run and the parity harness fixture exercises few of these
mechanisms (see the harness note at the end of each audit).

## Merged PRs

| PR | Row | Mechanism |
|---|---|---|
| #242 | 09.03 | One density level per harvest bite (`Harvest_Ore_Tick` 0x0073D450) |
| #243 | 09.04 | Skirmish growth reload `ftol(Growth × 0.3)` (0x00722C40) |
| #244 | 09.01 | `MultiplayerAICM` opening grant; leftover starting-unit budget |
| #245 | 09.05 | Own-house refinery selection, contact-slot passes, Sqrt_Approx too-far test |
| #246 | 07.39 | Refinery smoke/SpecialAnim on the dump gates; contact-gone abort |
| #247 | 15.07 | THEMEMD-only catalog, LOADING hand-off, Next_Song/Is_Allowed |
| #248 | 07.15 | No-ore park on Guard, `House+0x242`, war-miner HELLO at five cells |
| #249 | 15.06 | VoxClass priority queue with the 500 ms gap |
| #250 | 15.06 | Funds/low-power/unit-lost/under-attack predicates |
| #251 | 07.21 | Transport and Nighthawk Unload mission |
| #253 | 07.38 | Repair-depot admission by radio contact (no FIFO) |
| #254 | 15.06 | Sidebar, placement, capture, superweapon and defeat lines |
| #255 | docs | Research corrections, label sweep, System Map rows |
| #256 | 09.01 | Derrick cash, disc drain, slave whole-slot deposit, credit tick |
| #257 | audit | Depot probe RNG order, refinery-killed undock, purifier count, harvester re-idle on Move |
| #259 | audit | Harvester sent to a depot runs the depot probe (regression from #257) |
| #264 | audit | Harvester re-idle on owner change and depot exit; residual notes |

Closed by evidence, no code: GSI-09.02 (stock YR never fills building or
house storage; every retail map sets `FillSilos=no`) and GSI-07.17 (slot
`+0x234` = `0x005B2ED0` in all eight vtables, no assigner).

## Scan claims corrected during the phase

Read the scans with these corrections; the audits carry the evidence.

- Insufficient-funds nag re-arms at `SpeakDelay × 900.0` (0x007E27F8), not 792.25.
- Own force-fire on a building does announce under attack (no attacker-house test).
- Aircraft self-destruct/paradrop despawn is silent (`AircraftClass::Crash` 0x004DEBB0 never calls `Death_Announcement`).
- Radar event type 10 (capture) has no dedupe; sidebar OnHold/Canceled use type −1.
- The repair-depot 0x22 → 0x17 occupant-eviction loop belongs to the Hospital/Armory branch (0x0043CB0C) and is unreachable for `UnitRepair` depots; `Rules+0x16F8` is a constant 1.0, not an INI key.
- The Slave Miner drains the slave's own storage (0x00522D55), never the master building's; `+0x16CC` is `OrePurifier`, `+0x538C` the purifier building count.
- The disc money drain gates on `ResourceDestination=` (+0x5ED), not `Drainable=` (+0x5EF).
- `Type+0xC95` is `IsDropship=`; `AirportBound=` is `AircraftType+0xE0D`.
- A refused aircraft ejection natively loses the passenger; VERA keeps it and leaves for Guard (labelled).

## Remaining residuals

See `reverse-audit-phase7-final.md`, Part C. R1 was closed by #259 and R2 by
#264; the rest stay labelled in code with trigger, effect and address.
