# Refinery Pad Link / Occupancy Lifecycle - Implementation Verification Ghidra Report

**Address(es):** `BuildingClass::Receive_Radio @ 0x0043C2D0`, `UnitClass::Mission_Deploy_Building @ 0x0073D630`, `BuildingClass::ReleaseDockedHarvester @ 0x004595C0`, `RadioClass::Receive_Radio @ 0x0065A820`, `TechnoClass::Receive_Radio @ 0x006F4AB0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** stock `CMIN/HARV -> GAREFN/NAREFN` zero-link refinery unload path only: whether gamemd has a field/list equivalent to Rust `dock_reservations.on_pad` / `link_on_pad`, whether `+0x2E4` is written, and what should own unload-active occupancy.
**Non-Scope:** non-stock bunkers/service depots/aircraft pads, full sound/anim timing, full two-miner runtime frame order, and Rust implementation edits.
**Confidence:** High for stock zero-link negative facts and Rust-facing handoff.
**Active in YR:** Yes for stock refinery unloading; conditional for reciprocal `+0x2E4` release helper, which is live code but not the normal stock `GAREFN/NAREFN` deposit path.

## 0. Working Notes

**Target question:** Does active `gamemd.exe` have any stock-refinery field or list equivalent to Rust `dock_reservations.on_pad` / `link_on_pad`, and exactly when is it set/cleared if present?

**Non-goals:** Do not rediscover the settled `NW+(3,1)` accepted target, `NW+(2,1)` `GetDockCoord`, `NW+(4,1)` `QueueingCell`, or `0x15` receiver side-effect split unless a direct contradiction appears.

**Evidence needed to mark COMPLETE:** Decompile plus assembly/disassembly context for stock `0x15`, `Mission_Deploy_Building` entry/start/state-4, `ReleaseDockedHarvester`, `RadioClass` Contacts add/remove, and `TechnoClass +0x418`; stock INI/default proof for `DockUnload`, `Refinery`, and `Harvester`; current Rust scan for `link_on_pad`, `release_on_pad`, and occupancy predicates.

**Stop conditions:** Stop once stock `+0x2E4` write/no-write is proven, the active occupancy owners are identified, and the Rust `on_pad` mapping is classified as matched/mismatched/unchecked.

## 1. Bottom Line

There is no verified stock `gamemd.exe` physical-pad field/list equivalent to Rust `on_pad` in the normal `CMIN/HARV -> GAREFN/NAREFN` unload path. Stock refinery unloading is a zero-`+0x2E4` path:

- `0x15` queues sender mission `0x10` only; it does not write `unit+0x2E4`, `building+0x2E4`, coordinates, or pad occupancy.
- `UnitClass::Mission_Deploy_Building` immediately branches on `unit+0x2E4`; stock unload stays on the zero branch.
- unload-active state is `unit+0x6D1 = 1` plus mission substate `unit+0xBC = 3`, with the refinery rediscovered by adjacent-cell lookup when needed.
- refinery admission/serialization is owned by `RadioClass` Contacts (`+0xE4/+0xE8`) and the `Techno+0x418` contact flag, not a separate physical pad slot.

Rust `start_unload_deploy -> link_on_pad` is therefore not byte-field-equivalent to a stock gamemd pad link. If Rust keeps a separate map, it should be treated as Rust-internal `unload_active` bookkeeping, not as a physical pad/link field and not as `+0x2E4`.

## 2. Key Offsets And Owners

| Field / list | Owner | Verified role | Active in YR |
|---|---|---|---|
| `RadioClass +0xE4` | Techno/Building radio base | Contacts array pointer; HELLO stores sender, BREAK clears matching slot | Yes |
| `RadioClass +0xE8` | Techno/Building radio base | Contacts slot count/capacity | Yes |
| `Techno +0x418` | Unit and Building endpoints | radio-contact flag set by `0x18`, cleared by `0x19` cascade | Yes |
| `Unit +0x2E4` | Unit | conditional reciprocal dock link selector at `Mission_Deploy_Building` entry | Conditional; not stock zero-link unload |
| `Building +0x2E4` | Building | conditional linked harvester pointer used by `ReleaseDockedHarvester` | Conditional; not stock zero-link unload |
| `Building +0x718` | Building | conditional release-helper state cleared with `Building+0x2E4` | Conditional; not stock zero-link unload |
| `Unit +0x6D1` | Unit | unload-active latch set by mission `0x10`, cleared in state 4 | Yes |
| `Unit +0xBC` | MissionClass/Unit | mission substate; stock unload init writes `3` | Yes |
| `Unit +0xF8` | Unit | dump accumulator; zeroed at unload init | Yes |
| `Unit +0x100..+0x10C` | Unit | timer fields initialized at unload init | Yes |

## 3. Core Logic

### 3.1 Stock `0x15` does not set pad/link occupancy

`BuildingClass::Receive_Radio(0x15)` reaches the stock `DockUnload=yes` branch by testing `BuildingType+0x16B3`.

Assembly context:

- `0x0043C788`: `MOV CL,byte ptr [EAX + 0x16b3]`
- `0x0043C790`: zero branch skips stock DockUnload path
- `0x0043C796..0x0043C7A0`: loads sender, pushes `0`, pushes mission `0x10`, calls sender vtable `+0x1E8`
- `0x0043C7A6..0x0043C7B2`: returns `1`

**Active in YR:** Yes. Stock `[GAREFN]` and `[NAREFN]` set `DockUnload=yes`.

Material negative facts in this exact stock branch:

- no `unit+0x2E4` write;
- no `building+0x2E4` write;
- no Contacts mutation;
- no coordinate write/snap;
- no `+0x6D1`, `+0xF8`, or `+0xBC` write;
- no sound or anim call.

### 3.2 Mission `0x10` explicitly splits zero-link vs reciprocal-link paths

`UnitClass::Mission_Deploy_Building @ 0x0073D630` starts with the decisive branch:

- `0x0073D63B`: compare `dword ptr [ESI + 0x2E4]` with zero.
- `0x0073D641`: if zero, jump to the ordinary deploy/unload body.
- `0x0073D649..0x0073D66D`: if nonzero, look up a building and call `BuildingClass::ReleaseDockedHarvester`.

**Active in YR:** Yes for the function. The nonzero branch is conditional; the stock `0x15` chain does not create the required `+0x2E4` link.

This proves `+0x2E4` is not an implicit stock pad occupancy field. It is a branch selector for a different reciprocal-link cleanup path.

### 3.3 Stock unload-active occupancy is `+0x6D1` plus mission substate, not `on_pad`

On the zero-link harvester branch, after path/facing gates:

- `0x0073DEE0..0x0073DFBC`: valid path and facing/rate gate.
- `0x0073DFD0`: `Unit+0xF8 = 0`.
- `0x0073DFDA`: `Unit+0x6D1 = 1`.
- `0x0073DFE0..0x0073DFFC`: initialize timer fields around `+0x100..+0x10C`.
- `0x0073E013..0x0073E08E`: if `Harvester=yes`, rediscover adjacent refinery and set anim slot 7 if found.
- `0x0073E093`: `Unit+0xBC = 3`.

**Active in YR:** Yes for stock `HARV/CMIN`, because `Harvester=yes` is set and the stock path reaches mission `0x10`.

This is the closest binary-side equivalent to "unload active." It is unit mission state, not building pad occupancy and not reciprocal link state.

### 3.4 Stock state-4 cleanup clears unload-active and radio/contact state, not a pad link

The zero-link state-4 branch:

- checks adjacent refinery/anim state;
- `0x0073E1F6`: clears `Unit+0x6D1`;
- queues/continues harvest mission `0x0A`;
- may send `BREAK(3)` via radio protocol before mission commence, depending on path state.

`RadioClass::Receive_Radio(3)` removes the sender from Contacts by nulling the matching entry:

- `0x0065A895`: loads Contacts array from `+0xE4`;
- `0x0065A8A0`: writes zero into the matching contact slot.

`TechnoClass::Receive_Radio(0x19)` clears `+0x418`:

- `0x006F4BA6`: `MOV byte ptr [ESI + 0x418],0`
- `0x006F4BAD`: propagates `0x19`

**Active in YR:** Yes for Contacts/BREAK/Techno radio handling; exact post-unload cleanup frame is runtime-sensitive, but the owner fields are verified.

### 3.5 `ReleaseDockedHarvester` is the conditional reciprocal-link helper

`BuildingClass::ReleaseDockedHarvester @ 0x004595C0` is live but not stock zero-link unload. It reads `Building+0x2E4`; if non-null and the linked object is a unit, it:

- clears `unit+0x2E4`;
- force-tracks the unit out;
- sets a passable destination and move mission;
- clears `building+0x2E4` and `building+0x718`;
- queues building mission `5`;
- sends `BREAK(3)`.

**Active in YR:** Conditional. It requires a preexisting reciprocal link that the stock refinery `0x15` path does not write.

## 4. INI Keys

| INI key | Stock value | Effect | Active in YR |
|---|---|---|---|
| `[GAREFN] DockUnload` | `yes` (`ini/rulesmd.ini:11726`) | enables stock receiver `0x15` queue-mission branch | Yes |
| `[NAREFN] DockUnload` | `yes` (`ini/rulesmd.ini:12519`) | same | Yes |
| `[GAREFN] Refinery` | `yes` (`ini/rulesmd.ini:11727`) | later refinery/dump/anim classification | Yes |
| `[NAREFN] Refinery` | `yes` (`ini/rulesmd.ini:12520`) | same | Yes |
| `[CMIN]/[HARV] Harvester` | `yes` (`ini/rulesmd.ini:7314`, `7364`, `8184`, `8228`) | selects stock harvester mission `0x10` branch | Yes |
| `[CMIN]/[HARV] Dock` | `NAREFN,GAREFN` | target refinery list | Yes |
| `artmd.ini QueueingCell=4,1` | stock refinery art | waiting/fallback staging only, not `on_pad` | Yes |

## 5. Current Rust Implementation Status

| Rust surface | Current behavior observed | Binary delta / risk |
|---|---|---|
| `src/sim/miner/miner_dock.rs:26` | comment says `on_pad` is physical pad occupancy for stock unload/release bookkeeping | No verified stock physical-pad field/list exists; this wording is too strong |
| `src/sim/miner/miner_dock.rs:92` | `link_on_pad(refinery, miner)` inserts into `on_pad` | No stock `+0x2E4`/pad equivalent is written at unload start |
| `src/sim/miner/miner_dock.rs:120` | `pad_occupied` checks only `on_pad` | gamemd admission is Contacts-driven; no separate pad occupancy check was verified |
| `src/sim/miner/miner_dock.rs:185` | `is_occupied` considers Contacts, `contact_entered`, and `on_pad` | Contacts and contact flag are verified; `on_pad` is Rust-internal if retained |
| `src/sim/miner/miner_dock_sequence.rs:805` | `start_unload_deploy` calls `link_on_pad` when mission deploy starts | Should not be described as stock pad link or `+0x2E4`; closest verified state is unload-active `+0x6D1` |
| `src/sim/miner/miner_dock_sequence.rs:977` | `phase_departing` calls `release_on_pad` then `release_contact` | `release_contact` maps to Contacts/contact cleanup; `release_on_pad` has no direct stock field unless renamed to Rust unload-active cleanup |
| `src/sim/miner/mod.rs:103` | `FaceSync` doc correctly says no on-pad side effects | Matches latest contract |
| `src/sim/miner/mod.rs:108` | `MissionQueued` doc correctly says `0x15` has no pad occupancy side effects | Matches latest contract |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Stock `BuildingClass::Receive_Radio(0x15)` branch | verified | decompile `0x0043C2D0`; assembly `0x0043C788..0x0043C7A6` | none |
| Stock `0x15` no `+0x2E4` write | verified | same branch; no store besides sender mission queue call | none |
| `Mission_Deploy_Building` `unit+0x2E4` entry split | verified | decompile `0x0073D630`; assembly `0x0073D63B..0x0073D66D` | none |
| Unload-active init | verified | `0x0073DFD0`, `0x0073DFDA`, `0x0073DFE0..0x0073DFFC`, `0x0073E093` | exact sibling sound/anim timing outside scope |
| Zero-link state-4 unload cleanup | verified | `0x0073E1F6`, `0x0073E24D..0x0073E289` | exact runtime frame for BREAK cleanup deferred |
| `RadioClass` Contacts[] add/remove ownership | verified | decompile `0x0065A820`; assembly `0x0065A895..0x0065A8A0` | none |
| `Techno+0x418` contact flag set/clear | verified | decompile `0x006F4AB0`; assembly `0x006F4B72`, `0x006F4BA6` | none |
| `BuildingClass::ReleaseDockedHarvester` reciprocal-link helper | verified | decompile `0x004595C0`; caller branch `0x0073D63B..0x0073D66D` | non-stock producers outside scope |
| Rust `on_pad` mapping | verified scan | `rg link_on_pad/release_on_pad/on_pad` and source lines in section 5 | implementation patch separate |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-001 - Does stock refinery radio 0x15 write unit/building +0x2E4? -> No; it only calls sender vtable +0x1E8 with mission 0x10 and flag 0.` (evidence: `0x0043C788..0x0043C7A6`)
- `[RESOLVED] OQ-002 - Does Mission_Deploy_Building require +0x2E4 for stock unload? -> No; zero +0x2E4 is the ordinary branch, nonzero +0x2E4 calls ReleaseDockedHarvester first.` (evidence: `0x0073D63B..0x0073D66D`)
- `[RESOLVED] OQ-003 - What binary state represents unload-active? -> `Unit+0x6D1=1`, timer fields, and substate `+0xBC=3` after path/facing gates.` (evidence: `0x0073DFD0..0x0073E093`)
- `[RESOLVED] OQ-004 - What owns refinery admission occupancy? -> RadioClass Contacts[]; HELLO adds sender to `+0xE4` slot when capacity exists, BREAK clears the matching slot.` (evidence: `0x0065A820`)
- `[RESOLVED] OQ-005 - What owns contact-entered state? -> Techno byte `+0x418`, set by `0x18` and cleared by `0x19` cascade.` (evidence: `0x006F4B72`, `0x006F4BA6`)
- `[RESOLVED] OQ-006 - Does stock state-4 cleanup clear a physical pad field? -> No verified physical pad field is present; it clears `+0x6D1` and uses mission/radio cleanup.` (evidence: `0x0073E1F6`, `0x0065A820`, `0x006F4AB0`)
- `[RESOLVED] OQ-007 - Is ReleaseDockedHarvester normal stock refinery exit? -> No; it is reached only from the nonzero `unit+0x2E4` branch.` (evidence: `0x0073D63B..0x0073D66D`, `0x004595C0`)
- `[RESOLVED] OQ-008 - Should Rust `on_pad` be treated as `+0x2E4`? -> No; `+0x2E4` is not written in stock zero-link unload, and `on_pad` has no direct stock field equivalent.` (evidence: binary addresses above; Rust scan `src/sim/miner/miner_dock.rs:92`)
- `[DEFERRED] OQ-009 - Exact frame when normal post-unload BREAK/0x19 cleanup fires in a retail replay.` (category: needs-runtime-debugger; reason: static code proves owner fields and branches, not a concrete replay frame; next-step-if-pursued: runtime trace two CMIN one refinery unload completion)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock unload does not create reciprocal unit/building `+0x2E4` or pad link | `0x0043C788..0x0043C7A6`; `0x0073D63B..0x0073D66D` | `link_on_pad` creates a separate `on_pad` map at unload start | `src/sim/miner/miner_dock_sequence.rs::start_unload_deploy`; `src/sim/miner/miner_dock.rs` | Remove it for stock path, or rename/reclassify it as Rust-internal unload-active bookkeeping only | full stock miner deposit reaches unload without any `is_on_pad` assertion becoming required for correctness | Do not map `on_pad` to `+0x2E4`, physical pad arrival, or radio `0x15` |
| Refinery admission/serialization is Contacts[]-owned | `RadioClass::Receive_Radio @ 0x0065A820` | Rust has `contacts`, plus extra `pad_occupied` gate | `RefineryDockContacts::hello_or_wait`, `pad_occupied`, `phase_mission_enter` | Contacts/contact flag should be sufficient to block another stock miner until cleanup; `pad_occupied` should not be a stock CAN_DOCK gate unless proven elsewhere | two miners: second miner only enters after first contact cleanup and its own MissionEnter retry | Do not promote/deny miners based on an invented pad slot when Contacts[] already owns capacity |
| Unload-active is unit-side `+0x6D1` plus mission substate 3 | `0x0073DFD0..0x0073E093` | Rust lacks a named binary-equivalent latch; `on_pad` partly acts like one | miner component / dock phase FSM | If a separate field is retained, name/test it as unload-active, set it at mission deploy start, clear it at state-4 cleanup | after mission `0x10` path/facing gate, visual/unload state begins; before that no cargo drain/sound/pad side effects | Do not call it physical occupancy or use it as a coordinate/link proof |
| State-4 cleanup clears unload-active and contact through radio/contact protocol | `0x0073E1F6`; `0x0065A895..0x0065A8A0`; `0x006F4BA6` | Rust `release_contact` clears Contacts/contact_entered; `release_on_pad` clears extra map | `phase_departing`, `RefineryDockContacts::release_contact` | Keep contact cleanup separate from reciprocal-link release; if `on_pad` removed, state-4 still must clear Contacts/contact flag | after cargo-empty gate, refinery becomes available only through normal contact cleanup, not ReleaseDockedHarvester | Do not call `ReleaseDockedHarvester`, Force_Track(0x47), or reciprocal link cleanup for healthy stock completion |
| `ReleaseDockedHarvester` is conditional nonzero-link behavior | `0x004595C0`; caller `0x0073D63B..0x0073D66D` | Rust interrupt helpers still use `on_pad`/force-track concepts for conditional paths | `interrupt_refinery_docked_miners`, legacy release helpers | Keep this path only for verified nonzero-link/interrupt/non-stock contexts | destroy/sell/conditional linked-dock tests remain separate from healthy stock deposit | Do not let normal stock deposit depend on conditional release-helper semantics |

## 9. Negative Facts / Do Not Do

- Do not write or synthesize `unit+0x2E4` / `building+0x2E4` for stock `GAREFN/NAREFN` miner deposit.
- Do not describe `start_unload_deploy -> link_on_pad` as a gamemd pad/link handoff.
- Do not use `on_pad` as the stock physical position field; the miner remains governed by ordinary unit coordinates and mission state.
- Do not use `on_pad` as the stock receiver-side admission owner; `RadioClass` Contacts[] owns that role.
- Do not call `ReleaseDockedHarvester` or use Force_Track(0x47) for healthy stock cargo-empty completion.
- Do not treat `0x15`, `0x18`, `+0x418`, `+0x6D1`, and `+0x2E4` as interchangeable "docked" state. They are separate mechanisms.

## Sources

- Ghidra read-only decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`.
- Ghidra assembly context: `0x0043C788..0x0043C7A6`.
- Ghidra read-only decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`.
- Ghidra assembly context: `0x0073D63B..0x0073D66D`, `0x0073DFD0..0x0073E093`, `0x0073E1F6..0x0073E289`.
- Ghidra read-only decompile: `BuildingClass::ReleaseDockedHarvester @ 0x004595C0`.
- Ghidra read-only decompile: `RadioClass::Receive_Radio @ 0x0065A820`.
- Ghidra assembly context: `0x0065A895..0x0065A8A0`.
- Ghidra read-only decompile: `TechnoClass::Receive_Radio @ 0x006F4AB0`.
- Ghidra assembly context: `0x006F4B72`, `0x006F4BA6`.
- INI checked: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`, `ini/artmd.ini`, `ini/rules.ini`, `ini/art.ini`.
- Rust scanned: `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_dock.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_dock_sequence.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/mod.rs`.
- Related prior reports consulted: `RADIO_0X15_START_UNLOAD_SIDE_EFFECTS_GHIDRA_REPORT.md`, `RADIO_0X18_CONTACT_FLAG_LIFECYCLE_GHIDRA_REPORT.md`, `BUILDING_RECEIVE_RADIO_DOCK_CLEARANCE_HANDOFF_EXIT_GHIDRA_REPORT.md`.
