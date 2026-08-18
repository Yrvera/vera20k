# Radio Contact Lifecycle System Model Synthesis

**Date:** 2026-05-28  
**Type:** model-synthesis  
**Scope:** active YR radio/contact lifecycle across Building/Techno/Unit surfaces: `Contacts[]`, HELLO/BREAK (`0x02/0x03`), endpoint byte (`0x18/0x19` / `Techno+0x418`), stock refinery unload handoff (`0x15`, mission `0x10`), cleanup bridge (`0x08`), returned vs sent `0x17`, building change-owner, death, and sell cleanup.  
**Non-scope:** full scheduler timing, full ReceiveDamage semantics, generic Planning Mode, full war-factory production, aircraft docking, or render/audio presentation after these state writes.

## Research Index Preflight

`tools/research_index/brief.py` was run for radio/contact lifecycle terms. The index validated and pointed to the radio/refinery/building reports used below, but the map was noisy, so the synthesis uses direct report and address evidence.

## Evidence Ladder

- `BINARY_HIGH`: live Ghidra spot-checks of `RadioClass::Transmit_Radio_Impl @ 0x0065A970`, `TechnoClass::Receive_Radio @ 0x006F4AB0`, `BuildingClass::ChangeOwner @ 0x00448260` contact slice, and `BuildingClass::ReceiveDamage @ 0x00442230` death-contact slice, plus stock INI gates.
- `RESEARCH_HIGH`: 2026-05-28 re-swarm reports with exact addresses and Active-in-YR status.
- `DOC_SYNTHESIS`: older overview docs and current Rust touchpoint scans; used only for impact, not as behavioral proof.

## Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safe? |
|---|---|---:|---:|---:|---|
| `Contacts[]` is mutated by HELLO/BREAK (`0x02/0x03`), not `0x18/0x19`. | `0x0065A970`; `BUILDING_RADIO_0X18...` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `0x18` sets mirrored endpoint byte `Techno+0x418`; already-set falls through and returns `0`; `0x19` clears it. | `0x006F4B72`, `0x006F4BA6` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Building sends `0x18` only after a `0x12` reply of `0x14`, ignores `0x18` return, then proceeds to `0x16`. | `BUILDING_RADIO_0X18...` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Stock refinery `0x15` queues sender mission `0x10` and returns `1`; it does not start unloading or write pad/contact fields. | `0x0043C788..0x0043C7B2` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Accepted mission `0x10` unload start writes `+0xF8`, `+0x6D1`, timer cluster, optional anim slot, then state `3`; first drain is later. | `0x0073DFD0..0x0073E093` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| State 4 clears `+0x6D1`, assigns Harvest, then conditionally sends BREAK (`0x03`), not `0x08`. | `0x0073E1F6`, `0x0073E24F..0x0073E279` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Per-cell `0x08` is later cleanup: Techno receives `0x08`, sends `0x19`, then `0x03`. | `0x0073A93D`, `0x006F4C34..0x006F4C41` | confirmed | high | conditional | IMPLEMENTATION_SAFE |
| Stock refineries do not return `0x17` from `0x08`; factory/repair/bunker receivers may. | `0x0043C2D0`; `rulesmd.ini` gates | confirmed | high | yes/conditional | IMPLEMENTATION_SAFE |
| Returned `0x17` from `0x08` is a reply code consumed by selected senders, not a sent radio message. | `RADIO_0X08_0X17...` | confirmed | high | conditional | IMPLEMENTATION_SAFE |
| Sent `0x17` producers are building sell, death far-contact, change-owner eviction, and legacy Hospital/Armory cleanup. | `SENT_RADIO_0X17...`; `COMPUTED_SENT...` | confirmed | medium-high | yes/conditional | IMPLEMENTATION_SAFE for listed set |
| Building change-owner retains eligible contacts before building base transfer; failed retention sends `0x17` then `0x03`. | `0x00448566..0x004486D9` | confirmed | high | conditional | IMPLEMENTATION_SAFE |
| Building death sends `0x17` only to remaining far non-helipad contacts; close or helipad contacts take C4 damage. | `0x00442586..0x004425EE` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Exact same-frame admission of a waiting second miner after release is runtime-order conditional. | `HARV_POST_UNLOAD...` | unknown for full timing | medium | yes | NEEDS_REINVESTIGATE |

## Current Model

The radio/contact lifecycle has three separate state planes that must not be collapsed.

1. **Radio contact list:** `RadioClass+0xE4/+0xE8` stores sparse `Contacts[]`. HELLO (`0x02`) inserts only after receiver `ROGER(1)`. BREAK (`0x03`) removes matching sender slots before forwarding to the target receiver. Capacity, not live count, is the loop bound.
2. **Endpoint byte:** `Techno+0x418` is set and cleared by `0x18/0x19`. It can cascade during BREAK and `0x08` cleanup, but it is not the contact list and is not `Building+0x2E4`.
3. **Unload latch:** unit `+0x6D1` is the unload-active/render latch. Stock `0x15` does not set it. Accepted mission `0x10` sets it; state 4 clears it.

The stock `HARV/CMIN -> GAREFN/NAREFN` path is:

1. HELLO/admission owns `Contacts[]`.
2. Building can-dock flow sends `0x12`; on `0x14`, building sends `0x18`, ignores its return, then sends `0x16`.
3. Unit `0x16` gates may send `0x15` to the building.
4. Building `0x15` queues sender mission `0x10` only.
5. Mission `0x10` starts unloading only after its facing/path gates pass.
6. State 3 drains cargo later.
7. State 4 clears `+0x6D1`, assigns Harvest, then conditionally sends direct BREAK (`0x03`). A later per-cell `0x08` is catch-up cleanup only when contact-entered state survives.

`0x08` and `0x17` split into two concepts:

- Returned `0x17`: a reply from `BuildingClass::Receive_Radio(0x08)` for `WeaponsFactory`, `UnitRepair`, or `Bunker` paths, consumed by specific senders such as `UnitClass::PerCellProcess`.
- Sent `0x17`: an actual directed radio message from building sell, building death far-contact, building change-owner eviction, and default-off legacy Hospital/Armory cleanup. These are not interchangeable.

Building lifecycle cleanup:

- **Change-owner:** sparse contacts are classified before building base owner transfer. Retention requires non-null active contact, `0x13 == ROGER`, and either `WeaponsFactory=yes` or distance `<0x40`. Failed retention sends directed `0x17`, then directed `0x03`. Retained contacts change owner first, then are re-HELLOed after building owner/list work, restoring `+0x418` only if the saved byte was nonzero.
- **Death:** the current contact list is snapshotted. The exact linked `Building+0x2E4` occupant is removed before the remaining list is classified. Close contacts (`distance <0x100`) and all contacts of a dying `Helipad=yes` building take target `Strength*10` damage with `C4Warhead`. Far non-helipad contacts receive sent `0x17`, then caller clears `target+0x500`.
- **Sell:** state 0 broadcasts sent `0x17` through the building contact list before later BREAK cleanup.

## Implementation-Safe Facts

- Model `Contacts[]`, `Techno+0x418`, `Building+0x2E4`, and unit `+0x6D1` as distinct concepts.
- Implement HELLO/BREAK list mutation with sparse slot/capacity semantics if pursuing byte-level radio parity.
- Preserve `0x15` as a queued-mission boundary with no immediate unload, cargo, sound, coordinate, pad, or contact side effects.
- Preserve stock state-4 cleanup order: wait guard, clear `+0x6D1`, assign Harvest, then direct BREAK (`0x03`) if the branch gate passes.
- Treat stock refinery `0x08` as cleanup-only; do not use it for queue admission.
- Treat returned `0x17` and sent `0x17` as separate protocol facts.
- Implement building change-owner failed-contact eviction as `0x17` before `0x03`, not direct deletion.
- Implement building death contact routing with strict `<0x100` close gate and dying-building `Helipad=yes` override.

## Doc-Patch-Ready Facts

- Replace any claim that `0x18` owns HELLO/contact insertion with: `0x18` owns the mirrored endpoint byte only.
- Replace any claim that stock refinery admission uses `0x08 -> 0x17` with: stock `0x08` is cleanup; queue/admission retry belongs to miner `Mission_Enter` / `0x0E`.
- Replace any claim that post-unload state 4 sends `0x08` with: state 4 sends direct BREAK (`0x03`); later per-cell `0x08` is conditional cleanup.
- Replace any prose treating returned `0x17` from `0x08` as a sent radio message.
- Replace owner-change uncertainty with the decoded retention gates and `0x17`-then-`0x03` eviction order.

## Stale Or Superseded Claims

- **Stale:** "`0x18` inserts refinery contacts." Superseded by `RadioClass::Transmit_Radio_Impl @ 0x0065A970` and `TechnoClass::Receive_Radio @ 0x006F4AB0`.
- **Stale:** "`0x15` starts stock unload side effects." Superseded by `0x0043C788..0x0043C7B2` and mission `0x10` start evidence.
- **Stale:** "State 4 uses `0x08` for stock cleanup." Superseded by `0x0073E275..0x0073E279`.
- **Stale:** "Building death sends `0x17` to every contact." Superseded by death-contact close/helipad gates at `0x00442586..0x004425EE`.

## Cross-Doc Conflicts

No unresolved broad conflict remains for the static protocol facts above. The main open conflict is not conceptual but timing-related: the exact runtime frame when a waiting miner retries/admittance after another miner's state-4 release depends on live-vector order and mission timers.

## Needs Re-Investigation

- `/re-investigate stock refinery second miner admission exact frame after state-4 BREAK`
  - Needed because the static model proves ownership and ordering but not every runtime frame outcome.
- `/re-investigate Techno+0x500 after building death sent 0x17`
  - Needed to name the field and downstream users after the far-contact clear.
- `/re-investigate Object+0x14 bit 0x04 owner-change contact retention edge cases`
  - Needed if Rust models limbo/stale contacts byte-exactly across capture.
- `/trace-action destroy helipad with contacted aircraft far from pad`
  - Needed for player-visible damage/result confirmation through full target ReceiveDamage.

## Do-Not-Implement Notes

- Do not model `0x18` as HELLO or as contact-list insertion.
- Do not clear `+0x418` by cargo-empty or visual-state logic; clear it through `0x19`/BREAK-equivalent cleanup.
- Do not route stock refinery queue admission through `0x08 -> 0x17`.
- Do not turn returned `0x17` into a sent `0x17`.
- Do not clear all contacts on building capture.
- Do not evict owner-change contacts with BREAK only.
- Do not use a dense live-count Vec as a byte-parity replacement for sparse `Contacts[]`.
- Do not send `0x17` to close building-death contacts or to all helipad contacts; those use the damage branch.
- Do not treat exact same-frame waiter admission as implementation-safe until runtime-traced.

## Source Ledger

- `docs/research/BUILDING_RADIO_0X18_CONTACT_LIFECYCLE_RESWARM_20260528.md`
- `docs/research/BUILDING_RADIO_0X15_UNLOAD_SIDE_EFFECTS_RESWARM_20260528.md`
- `docs/research/HARV_POST_UNLOAD_RADIO_0X08_FRAME_ORDER_RESWARM_20260528.md`
- `docs/research/RADIO_0X08_0X17_FACTORY_REPAIR_BUNKER_SENDER_PATHS_RESWARM_20260528.md`
- `docs/research/SENT_RADIO_0X17_CALLER_SWEEP_RESWARM_20260528.md`
- `docs/research/COMPUTED_SENT_RADIO_0X17_DATAFLOW_SWEEP_RESWARM_20260528.md`
- `docs/research/BUILDING_CHANGEOWNER_CONTACT_RETENTION_0X17_0X03_RESWARM_20260528.md`
- `docs/research/BUILDING_DEATH_RADIO_0X17_CLOSE_HELIPAD_GATES_RESWARM_20260528.md`
- `docs/research/UNITCLASS_STATE4_VTABLE_0X200_IDENTITY_RESWARM_20260528.md`
- Ghidra spot-checks: `0x0065A970`, `0x006F4AB0`, `0x00448260`, `0x00442230`
- Stock INI checks: `ini/rulesmd.ini:818` `C4Warhead=Super`; `:11726/:12519` `DockUnload=yes`; `:11727/:12520` `Refinery=yes`; `:11729/:12521` `NumberOfDocks=1`; `:11775/:12565/:13309` `WeaponsFactory=yes`; `:11895/:12683/:13438/:13886` `UnitRepair=yes`; `:13732` `Bunker=yes`; `:11820/:12342` `Helipad=yes`
- Current Rust touchpoints: `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_tests.rs`, `src/sim/game_entity.rs`, `src/sim/combat/mod.rs`, `src/sim/aircraft/mod.rs`, `src/rules/object_type.rs`

**Overall status:** IMPLEMENTATION_SAFE for static radio/contact protocol and listed lifecycle branches; NEEDS_REINVESTIGATE for exact runtime frame admission and a few field semantic labels.
