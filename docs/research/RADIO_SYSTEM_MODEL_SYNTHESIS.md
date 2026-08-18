# Radio System Model Synthesis

Generated: 2026-05-22

Type: conflict-map plus implementation-safe model. The canonical radio overview is stale in
several places, but the newer focused radio reports and verify-doc audits agree on the main
protocol pieces.

## Scope

Included: RadioClass contact primitives, major radio opcodes recently investigated
(`0x0D`, `0x11`, `0x12`, `0x13`, `0x1C`, `0x22`), refinery/transport/airfield/service
handoffs, limbo/despawn contact cleanup, current Rust radio surfaces.

Non-scope: complete radio opcode table, full repair cost virtual internals, screenshot-level
transport ramp frames, broad campaign carryall activation proof, full computed-send census.

## Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safe? |
|---|---|---|---|---|---|
| RadioClass uses sparse contact slots and synchronous receiver dispatch via vtable `+0x194`. | `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`; audits; Ghidra `0x0065A970`, `0x0065A820` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| HELLO/BREAK manage reciprocal contact slots; BREAK clears sender and target side. | `GENERIC_DESPAWN...`; verify slot 5; Ghidra `0x0065ACE0`, `0x0065A970`, `0x0065A820` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Techno limbo broadcasts `BREAK(3)` before conceal. | `BROADCAST_RADIO_TO_ALL...`; verify slot 5; Ghidra `0x0065AA80` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `0x0D` is sent by `TechnoClass__ProcessCloakAndNotify` after successful mark when `Techno+0x418` is set. | re-swarm; verify slot 1; parent Ghidra `0x006F4A70` | confirmed | high | yes | DOC_PATCH_READY |
| Building receivers map non-swallowed `0x0D` through ObjectClass to vtable `+0x124(2)`; `WeaponsFactory=yes` swallows. | `BUILDING_VTABLE_0X124...`; re-swarm | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `0x11` is transport/passenger entry status polling, not refinery/nav. | `RADIO_MSG_0X11...`; verify slot 1; `TRANSPORT_DOOR...` | confirmed | high | conditional on passenger tracker | IMPLEMENTATION_SAFE for semantics |
| `DeployTime` tracker duration is `trunc(value * 900)`, so `.022` is 19 ticks. | `TRANSPORT_DOOR...` | confirmed | medium | conditional | DOC_PATCH_READY |
| Transport ramp/door exact rendered frame use is known. | `TRANSPORT_DOOR...` | unknown | low | conditional | NEEDS_REINVESTIGATE |
| `0x12` is FootClass `MOVE_TO_CELL`; already at target returns `0x14`, assigned move returns `1`. | `RADIO_0X12...`; verify slot 2; Ghidra `0x004D8FB0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Standard refinery sends `0x12` with building NW plus `(3,1)`, not art `QueueingCell`. | `RADIO_0X12...`; verify slot 2; `rulesmd.ini`, `artmd.ini` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `0x12` sender list in the focused report is exhaustive. | verify slot 2 | contradicted | high | conditional legacy | DOC_PATCH_READY |
| `0x13` is global NEED_TO_MOVE/readiness query, not dock-cell assignment. | `RADIO_0X13...`; verify slot 3; parent Ghidra `0x004D8FB0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| AircraftClass `0x13` return behavior is pure FootClass return behavior. | verify slot 3; parent Ghidra `0x004190B0` | contradicted | high | yes | DOC_PATCH_READY |
| Carryall LAND uses `0x13`, but stock normal skirmish carryall path is dormant behind unbuildable `[HIND] Carryall=yes; TechLevel=-1`. | `RADIO_0X13...`; `rulesmd.ini` | confirmed | medium | conditional/dormant | UNSAFE_FOR_IMPLEMENTATION beyond gate |
| Airfield docking is RadioClass contact-slot based; `CachedDock` is revalidated by `0x0F`. | `AIRFIELD_RADIO_CACHEDDOCK...`; verify slot 4 | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `NumberOfDocks` sets RadioClass contact capacity and multi-pad `DockingOffsetN` is selected by contact slot. | `AIRFIELD_RADIO_CACHEDDOCK...`; verify slot 4; `rulesmd.ini`, `artmd.ini` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| UnitReload buildings iterate contacted aircraft and send `0x1D`, `0x13`, `0x1F`, then `0x1C`. | `AIRFIELD_RADIO_CACHEDDOCK...`; verify slot 4 | confirmed | high | yes | IMPLEMENTATION_SAFE for ordering |
| `0x22` is read-only repair-needed query; `0x1C` is Techno repair tick, with Foot chrono rejection. | `SERVICE_REPAIR_RADIO...` | confirmed | high | yes | IMPLEMENTATION_SAFE for broad semantics |
| Repair depot sends `0x13` before `0x1C`. | `SERVICE_REPAIR_RADIO...` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Stock YR Hospital/Armory walk-in radio service is active normal behavior. | verify slot 1; `rulesmd.ini` commented keys | contradicted | high | no/legacy conditional | DOC_PATCH_READY |
| Current Rust generic `radio_contacts` cleanup is complete. | `GENERIC_DESPAWN...`; verify slot 5; Rust `rg` | contradicted | high | n/a | IMPLEMENTATION_SAFE gap |

## Current Model

Radio is not one gameplay subsystem. It is a shared contact and message protocol used by
several higher-level systems:

1. RadioClass owns contact slots and synchronous message delivery. HELLO creates sparse
   reciprocal contacts; BREAK clears sender-side and target-side contact state.
2. Most player-visible behavior is implemented by class-specific `Receive_Radio` switches:
   FootClass, UnitClass, AircraftClass, BuildingClass, TechnoClass, and ObjectClass differ.
3. The same opcode can have contextual meaning. `0x13` is a readiness/destination query in
   FootClass, but AircraftClass overrides the return after using the FootClass payload side
   effect. `0x12` is move-to-cell for FootClass, but payload shape varies by sender.
4. Stock refinery docking uses radio as an interaction sequence: readiness `0x13`, movement
   `0x12`, then follow-up messages after arrival. The accepted refinery movement cell is
   building NW plus `(3,1)`.
5. Airfield docking uses RadioClass contact capacity and contact-slot pad identity rather
   than a FIFO queue.
6. Service depot repair uses radio-style handoff before repair ticks. Hospital/Armory
   radio-service paths are legacy/conditional in stock YR.
7. Limbo/despawn cleanup is a radio concern: gamemd broadcasts BREAK before conceal/limbo.
   Rust currently has generic `radio_contacts` but no central reciprocal cleanup across all
   direct removal and limbo-like paths.

## Implementation-Safe Facts

- Implement `0x12` as synchronous `MOVE_TO_CELL`: already at payload target returns `0x14`;
  otherwise assign destination, write movement timing fields, and return `1`.
- For standard refinery `0x12`, use building NW plus `(3,1)`, not `QueueingCell`.
- Treat `0x13` as NEED_TO_MOVE/readiness, not dock-cell assignment.
- Do not implement AircraftClass `0x13` as a pure inherited FootClass return. It calls
  FootClass for payload then recomputes return.
- Keep war-factory `0x0D` as no production/overlay animation effect; `WeaponsFactory=yes`
  swallows it.
- Model airfield pads from RadioClass contact slots and `NumberOfDocks`, with `CachedDock`
  revalidated by `0x0F`.
- Add a deterministic reciprocal cleanup path for Rust `GameEntity.radio_contacts` before
  despawn/limbo-like transitions; include direct removal paths, passenger-inside transitions,
  and the current `aircraft/drop_payload.rs` retry surface.

## Doc-Patch-Ready Facts

- Patch `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`: `0x11` is transport passenger-entry polling,
  not only a harvester/refinery tail sender.
- Patch `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`: `0x13` should consistently be NEED_TO_MOVE /
  readiness, not `IS_UNIT_LINKED`.
- Patch `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`: close the `0x0D` sender open question with
  `TechnoClass__ProcessCloakAndNotify @ 0x006F4A70`.
- Patch `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`: Hospital/Armory radio service is
  legacy/conditional; stock `rulesmd.ini` comments out those keys.
- Patch `RADIO_0X12_MOVE_TO_CELL_PAYLOAD_AND_TIMESTAMPS_GHIDRA_REPORT.md`: sender list is
  not exhaustive because of the conditional legacy BuildingClass `0x0E -> 0x12` branch.
- Patch `RADIO_0X13_DOCKING_CELL_REQUEST_ROLES_GHIDRA_REPORT.md`: AircraftClass `0x13`
  has FootClass payload side effect but AircraftClass return semantics.
- Patch `GENERIC_DESPAWN_LIMBO_CLEANUP_ENTRY_POINTS_GHIDRA_REPORT.md`: replace the
  `passenger.rs:1055-1067` production-garrison citation with the combat/world production
  path, and add `src/sim/aircraft/drop_payload.rs` to the Rust cleanup surface inventory.

## Stale Or Superseded Claims

- Canonical radio doc claim that `0x11` is only harvester/refinery-related is superseded by
  `RADIO_MSG_0X11_SENDERS_AND_MEANING_GHIDRA_REPORT.md` and `TRANSPORT_DOOR...`.
- Canonical radio doc `0x13 = IS_UNIT_LINKED` wording is superseded by `RADIO_0X13...` and
  parent Ghidra spot-check of FootClass and AircraftClass.
- Canonical radio doc `0x0D` sender open question is superseded by `RADIO_MSG_0X0D...` and
  parent Ghidra spot-check of `0x006F4A70`.
- `0x007F05DC` as an unknown runtime caller is superseded by cleanup audit evidence: it is
  RadioClass vtable base `0x007F0508` plus `0xD4`, pointing to `0x0065AA80`.

## Cross-Doc Conflicts

- `FOOTCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` and
  `UNITCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` reportedly still use
  `REQUEST_DOCK_CELL / QUEUE_DOCK` wording for `0x13`. Current evidence supports
  NEED_TO_MOVE/readiness.
- The canonical radio overview is currently YELLOW after a prior patched-to-GREEN state.
  Use the focused newer reports for implementation handoff until the overview is patched.

## Needs Re-Investigation

- `/re-investigate transport door/deploy tracker visual frame consumption`
  to determine whether and how the 19-tick tracker affects rendered ramp/door frames.
- `/re-investigate radio 0x08 0x17 live sender coverage for factory repair bunker clearance`
  if implementing `0x08 -> 0x17` beyond the verified receiver-side cases.
- `/re-investigate service depot repair cost and repair-step virtual internals`
  before claiming exact money/health tick parity.
- `/re-investigate campaign/script carryall radio mission reachability`
  only if carryall behavior becomes implementation-relevant.

## Do-Not-Implement Notes

- Do not build one generic opcode table that ignores receiver class.
- Do not treat `0x13` as dock-cell assignment; cell assignment follows via `0x12`.
- Do not drive stock war-factory production animation from `0x0D`.
- Do not model airfield pad choice as FIFO if the goal is binary parity.
- Do not implement stock Hospital/Armory walk-in radio service as normal YR behavior.
- Do not assume dock reservation cleanup covers generic `radio_contacts` cleanup.

## Source Ledger

Primary reports:
- `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`
- `RADIO_MSG_0X11_SENDERS_AND_MEANING_GHIDRA_REPORT.md`
- `TRANSPORT_DOOR_TIMING_RADIO_0X11_DEPLOY_TRACKER_GHIDRA_REPORT.md`
- `RADIO_MSG_0X0D_SENDERS_ANIM_RESET_GHIDRA_REPORT.md`
- `BUILDING_VTABLE_0X124_RADIO_0X0D_VISUAL_DELTA_GHIDRA_REPORT.md`
- `RADIO_0X12_MOVE_TO_CELL_PAYLOAD_AND_TIMESTAMPS_GHIDRA_REPORT.md`
- `RADIO_0X13_DOCKING_CELL_REQUEST_ROLES_GHIDRA_REPORT.md`
- `AIRFIELD_RADIO_CACHEDDOCK_CONTACT_LIFETIME_GHIDRA_REPORT.md`
- `SERVICE_REPAIR_RADIO_0X1C_0X22_PATH_GHIDRA_REPORT.md`
- `GENERIC_DESPAWN_LIMBO_CLEANUP_ENTRY_POINTS_GHIDRA_REPORT.md`
- `RUST_RADIO_ABSTRACTION_GAP_SCAN_GHIDRA_REPORT.md`

Audit evidence:
- `AUDIT_LOG.md` entries from 2026-05-20 and 2026-05-22 for the above radio docs.
- `.swarm-claims.md` radio protocol fan-out blocks at 2026-05-22T14:36+02:00 and
  2026-05-22T14:58+02:00.

Parent spot-checks:
- Ghidra `0x004190B0` `AircraftClass__Receive_Radio`: confirms `0x13` return override.
- Ghidra `0x004D8FB0` `FootClass__Receive_Radio`: confirms `0x12` and `0x13` semantics.
- Ghidra `0x006F4A70` `TechnoClass__ProcessCloakAndNotify`: confirms `0x0D` sender.

INI/Rust checks:
- `ini/rulesmd.ini`: `[GAREFN]`, `[NAREFN]`, `[GAAIRC]`, `[AMRADR]`, `[HIND]`,
  commented `Hospital=yes` / `Armory=yes`.
- `ini/artmd.ini`: `[GAAIRC] DockingOffset0..3`.
- Rust surfaces: `src/sim/game_entity.rs`, `src/sim/world/mod.rs`,
  `src/sim/docking/aircraft_dock.rs`, `src/sim/miner/miner_dock.rs`,
  `src/sim/passenger.rs`, `src/sim/aircraft/drop_payload.rs`,
  `src/app_sim_tick.rs`.
