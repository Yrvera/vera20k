# Building Radio 0x18 Contact Lifecycle - Reswarm Report

**Address(es):** `BuildingClass__Receive_Radio @ 0x0043C2D0`, `TechnoClass__Receive_Radio @ 0x006F4AB0`, `RadioClass__Transmit_Radio_Impl @ 0x0065A970`, `RadioClass__Receive_Radio @ 0x0065A820`, supporting `UnitClass__Receive_Radio @ 0x00737430`, `FootClass__Mission_Enter @ 0x004D9290`, `ObjectClass__Receive_Radio @ 0x005F5320`.
**Investigation Mode:** exhaustive-slice.
**Claimed Scope:** refinery/dock/building contact lifecycle for radio message `0x18`, especially sender/receiver field writes, RadioClass contact-list interaction, reply codes, and timing relative to `0x12`, `0x14`, `0x16`, and `0x15`.
**Non-Scope:** full miner harvest FSM, full `0x15` unload/deposit FSM, accepted-cell coordinate proof beyond the `0x18` sender gate, and non-refinery building radio systems except where they distinguish stock YR activity.
**Confidence:** High for the stock `GAREFN/NAREFN` `DockUnload=yes` path and `0x18/0x19` field semantics; Medium for exact live-frame timing of later cleanup cascades after normal unload because that requires runtime tracing.
**Active in YR:** Yes for stock `CMIN/HARV -> GAREFN/NAREFN`. `rulesmd.ini` has `[CMIN] Dock=NAREFN,GAREFN` at line `7361`, `[HARV] Dock=NAREFN,GAREFN` at line `8225`, `[GAREFN] DockUnload=yes` / `Refinery=yes` at lines `11726..11727`, and `[NAREFN] DockUnload=yes` / `Refinery=yes` at lines `12519..12520`.

## 1. Bottom Line

Radio `0x18` is a Techno endpoint flag handshake, not a RadioClass contact-list insertion and not an unload-start event.

For the stock refinery admission burst:

1. A prior `HELLO(0x02)`/contact admission owns `RadioClass::Contacts[]`.
2. `FootClass::Mission_Enter` sends `CAN_DOCK(0x0E)` to its destination/contact.
3. `BuildingClass::Receive_Radio(0x0E)` refreshes/validates contact membership, computes the accepted cell, sends `0x12`, and requires reply `0x14`.
4. Only after `0x12 == 0x14`, the building sends directed `0x18` to the miner and ignores that call's return value.
5. `TechnoClass::Receive_Radio(0x18)` sets byte `Techno+0x418 = 1` only if currently zero, then transmits `0x18` back to the sender and returns `1`.
6. The propagated `0x18` sets the refinery/building endpoint's own `+0x418 = 1`, then pings back; the already-set endpoint falls through to `RadioClass/ObjectClass` and returns `0`, stopping useful propagation.
7. The building then sends `0x16`. Later `0x16` or per-cell branches may send `0x15`; `0x18` itself does not queue mission `0x10`, set pad occupancy, clear movement, write `+0x2E4`, or start cargo dumping.

The contact lifecycle is therefore two-layered:

- `RadioClass::Contacts[]` is populated/removed by `0x02` and `0x03`.
- `Techno+0x418` is set/cleared by `0x18` and `0x19`, normally over an existing radio contact.

## 2. Field And Message Map

| Field/message | Owner | Verified role | Evidence | Active in YR |
|---|---|---|---|---|
| `Contacts.data` | Radio/Techno `+0xE4` | Contact-pointer array used by `Transmit_Radio` and `ToFirst` | `RadioClass__Transmit_Radio_Impl` decompile; assembly context `0x0065A970` | Yes |
| `Contacts.capacity` | Radio/Techno `+0xE8` | Slot count scanned by `0x02`/`0x03`; stock refineries have `NumberOfDocks=1` | decompile `0x0065A970`; `rulesmd.ini` lines `11729`, `12521` | Yes |
| `RadioHistory` | Radio `+0xD4/+0xD8/+0xDC` | Updated only when execution reaches `RadioClass::Receive_Radio` | decompile `0x0065A820`; assembly context `0x0065A820` | Yes |
| `Techno+0x418` | Unit and Building endpoints | Mirrored dock/contact byte set by `0x18`, cleared by `0x19` | `0x006F4B72`, `0x006F4BA6` assembly contexts | Yes |
| `Techno+0x419` | Techno endpoints | Adjacent byte for `0x1A/0x1B`, not this path | `TechnoClass__Receive_Radio` decompile | Conditional, not stock refinery `0x18` |
| `Unit+0x5A4` | Foot/Unit | Destination pointer read with `+0x418` by later `0x16` / per-cell gates | `0x0073774A..0x0073777A`; related reports | Yes |
| `Unit+0x6D1` | Unit | DockUnload active latch; state-4 clears this, not `+0x418` | `0x0073E1F6` assembly context | Yes |
| `+0x2E4` | Techno/Building/Unit layouts | Not written by `0x18`, `0x19`, or stock refinery `0x15` | negative evidence in decompile `0x006F4AB0`; `0x0043C788..0x0043C7A0` prior report | No for this slice |
| `0x12` | Building -> Unit | Move/check accepted cell; `0x14` reply gates `0x18` | `0x0043CAB4..0x0043CAC1` | Yes |
| `0x14` | Unit -> Building reply | "already at the sent cell"; required before building emits `0x18` | `0x0043CABE..0x0043CAC1`; `0x004D9140..0x004D9197` prior report | Yes |
| `0x18` | Building/Techno directed | Set/propagate `+0x418` only; no contact-list mutation | `0x0043CAC7..0x0043CACE`; `0x006F4B72..0x006F4B79` | Yes |
| `0x16` | Building -> Unit | Sent immediately after `0x18`; may turn-sync or later cascade `0x15` | `0x0043CAD4..0x0043CAE4`; `0x007376BF..0x00737780` | Yes |
| `0x15` | Unit -> Building | DockUnload handoff; building queues sender mission `0x10` for stock refineries | `0x0043C788..0x0043C7A0` prior report | Yes |
| `0x19` | Techno directed | Clear/propagate `+0x418`; no direct `Contacts[]` mutation | `0x006F4BA6..0x006F4BAD` | Conditional |
| `0x03` | Radio break | Removes `Contacts[]`; Techno may send `0x19` first if flags are set | `RadioClass__Transmit_Radio_Impl`, `RadioClass__Receive_Radio`, `TechnoClass__Receive_Radio` | Conditional |

## 3. Verified Lifecycle

### 3.1 Contact admission precedes `0x18`

`0x18` does not create a radio contact. The contact list is owned by `RadioClass` `HELLO(0x02)` and `BREAK(0x03)`.

`RadioClass__Transmit_Radio_Impl @ 0x0065A970` has three relevant modes:

- If the target argument is null, it loads `Contacts[0]` from `+0xE4` and returns `0` if there is no target.
- For `0x03`, it scans all `Contacts[]` slots and nulls every slot matching the target before forwarding the message.
- For `0x02`, it scans for an existing target and returns `1` idempotently if already present; otherwise it finds a free slot or evicts slot 0 with `0x03`, forwards `0x02`, and writes the target into the chosen slot only if the receiver returns `1`.
- For every other message, including `0x18`, it directly forwards to the target receiver through vtable `+0x194` and does not insert, remove, resize, compact, or reorder `Contacts[]`.

**Evidence:** decompile `RadioClass__Transmit_Radio_Impl`; assembly context `0x0065A970..0x0065A97E` shows target defaulting to `Contacts[0]` for null target. The decompile shows special cases only for `param_2 == 3` and `param_2 == 2`; all other messages jump to the generic receive-dispatch path.

**Active in YR:** Yes. Stock refinery admission has an existing contact from the harvester/refinery radio approach before the accepted `0x0E` handoff sends `0x18`.

### 3.2 Building `0x0E` sends `0x18` only after `0x12 == 0x14`

In `BuildingClass__Receive_Radio @ 0x0043C2D0`, case `0x0E` handles stock `DockUnload=yes` and `Weeder=yes` admission. The relevant stock refinery order is:

1. Call/refresh base Techno radio behavior for `0x0E`.
2. Require building power.
3. Check or create the radio contact through `Contacts[]` membership and possible `HELLO(0x02)`.
4. For `DockUnload=yes` / `Weeder=yes`, compute the accepted cell from the building's cell plus `(+3,+1)`.
5. Write the cell payload and send `0x12`.
6. Compare the `0x12` reply to `0x14`.
7. If not `0x14`, jump away and return without `0x18` or `0x16`.
8. If `0x14`, send directed `0x18`, then directed `0x16`.

Assembly context for the handoff block:

```text
0043CAB2  PUSH EDI
0043CAB3  PUSH EBX
0043CAB4  PUSH 0x12
0043CAB8  CALL dword ptr [EAX + 0x27C]
0043CABE  CMP  EAX,0x14
0043CAC1  JNZ  0x0043CCF2
0043CAC9  PUSH EDI
0043CACA  PUSH 0x18
0043CACE  CALL dword ptr [EDX + 0x278]
0043CAD6  PUSH EDI
0043CAD7  PUSH 0x16
0043CADB  CALL dword ptr [EAX + 0x278]
0043CAE1  CMP  EAX,0x1
0043CAE4  JZ   0x0043CCF2
```

There is no compare of the `0x18` return value. The next checked return is `0x16`.

**Evidence:** decompile `BuildingClass__Receive_Radio`; `get_assembly_context` at `0x0043CAAE`, `0x0043CAC7`, and `0x0043CAD4`.

**Active in YR:** Yes. Stock `GAREFN/NAREFN` select the `DockUnload=yes` branch, and stock `CMIN/HARV` dock with them.

### 3.3 First receiver-side `0x18`: set, propagate, return `1`

`TechnoClass__Receive_Radio @ 0x006F4AB0` case `0x18`:

1. Checks an aircraft-specific `WhatAmI()==2` / type `+0xE0D` skip gate. Stock harvesters/refineries are not this aircraft path.
2. Reads `byte [this+0x418]`.
3. If zero, writes `1`.
4. Calls vtable `+0x278` with message `0x18` and the original sender.
5. Returns `1`, regardless of the propagated transmit's return.

Assembly context:

```text
006F4B59  MOV  AL, byte ptr [ESI + 0x418]
006F4B5F  TEST AL,AL
006F4B61  JNZ  0x006F4E3F
006F4B6D  PUSH EAX              ; sender/partner
006F4B6E  PUSH 0x18
006F4B72  MOV  byte ptr [ESI + 0x418],0x1
006F4B79  CALL dword ptr [EDX + 0x278]
006F4B81  MOV  EAX,0x1
006F4B8A  RET  0xC
```

The store happens before propagation. This matters because the immediate pingback from the partner sees the first endpoint already set.

**Evidence:** decompile `TechnoClass__Receive_Radio`; `get_assembly_context` at `0x006F4B72` and `0x006F4B79`.

**Active in YR:** Yes for unit and building endpoints in the stock refinery chain.

### 3.4 Mirrored endpoint effect and already-set return

The directed building send reaches the miner first. The miner sets `Unit+0x418 = 1` and propagates `0x18` back to the refinery. `BuildingClass` has no case `0x18`, so the refinery falls through to `TechnoClass__Receive_Radio(0x18)`, sets `Building+0x418 = 1`, and propagates `0x18` back to the miner.

The miner is now already set. In the already-set branch, `TechnoClass__Receive_Radio(0x18)` jumps to the generic fallback (`0x006F4E3F`), which calls `RadioClass__Receive_Radio`; that function handles only `0x02` and `0x03` specially and otherwise delegates to `ObjectClass__Receive_Radio`. `ObjectClass__Receive_Radio` handles only `0x0D` and `0x22`; for `0x18` it returns `0`.

Therefore:

- first `0x18` on a clear endpoint returns `1`;
- propagated `0x18` on a clear partner returns `1` to its local caller;
- final pingback to an already-set endpoint returns `0` through the fallback;
- the original building-side `0x0E` code ignores the directed `0x18` return anyway and still sends `0x16`.

**Evidence:** `TechnoClass__Receive_Radio` decompile and assembly `0x006F4B59..0x006F4B8A`; `RadioClass__Receive_Radio` decompile; `ObjectClass__Receive_Radio` decompile confirms all messages except `0x0D` and `0x22` return `0`.

**Active in YR:** Yes. This is the normal anti-ping-pong termination mechanism for a two-endpoint stock refinery `0x18` propagation.

### 3.5 RadioHistory detail: first-set `0x18` bypasses base receive

`RadioClass__Receive_Radio @ 0x0065A820` shifts `RadioHistory` at `+0xD4/+0xD8/+0xDC` before its `0x02`/`0x03` handling. A first-set `0x18` in `TechnoClass__Receive_Radio` returns directly after the `+0x418` write and propagated transmit, so it does not update `RadioHistory` on that endpoint through the base handler. The already-set pingback does fall through to `RadioClass__Receive_Radio`, so that fallback can record `0x18` before `ObjectClass` returns `0`.

**Evidence:** `TechnoClass__Receive_Radio` direct return at `0x006F4B81..0x006F4B8A`; `RadioClass__Receive_Radio` decompile and assembly context `0x0065A820..0x0065A833` show the history shift before message-specific handling.

**Active in YR:** Yes, but no gameplay reader of `RadioHistory` was found in prior `RADIOHISTORY_READ_USE_SCAN_GHIDRA_REPORT.md`; this is still a byte-state parity detail.

### 3.6 `0x19` clears the mirrored byte, not the contact list

`TechnoClass__Receive_Radio(0x19)` is the inverse byte handshake:

```text
006F4B8D  MOV  AL, byte ptr [ESI + 0x418]
006F4B93  TEST AL,AL
006F4B95  JZ   0x006F4E3F
006F4BA1  PUSH EAX
006F4BA2  PUSH 0x19
006F4BA6  MOV  byte ptr [ESI + 0x418],0x0
006F4BAD  CALL dword ptr [EDX + 0x278]
006F4BB5  MOV  EAX,0x1
```

It clears `+0x418` and propagates `0x19` only when the byte is currently nonzero. If already clear, it falls through to `RadioClass/ObjectClass`, so generic `0x19` returns `0`. Like `0x18`, `0x19` itself does not add or remove `Contacts[]`.

**Evidence:** decompile `TechnoClass__Receive_Radio`; `get_assembly_context` at `0x006F4BA6` and `0x006F4BAD`.

**Active in YR:** Conditional. The code is live, but the clean stock DockUnload state-4 exit does not directly send `0x19`; later cleanup/cancel/break paths can.

### 3.7 `0x03` and `0x08` are the contact cleanup bridge

The byte flag and contact array meet during cleanup:

- `TechnoClass__Receive_Radio(0x03)` checks this endpoint's `+0x418` and the sender's `+0x418`; when both are set, it sends `0x19` before falling into `RadioClass__Receive_Radio(0x03)`.
- `RadioClass__Transmit_Radio_Impl(0x03)` removes the target from the sender's `Contacts[]` before forwarding `0x03`.
- `RadioClass__Receive_Radio(0x03)` removes the sender from the receiver's `Contacts[]` and runs `ObjectClass::Receive_Radio` side effects.
- `TechnoClass__Receive_Radio(0x08)` sends `0x19` and then `0x03` through vtable `+0x278`.

Assembly context for the `0x08` clear/break cascade:

```text
006F4C2F  PUSH EDI
006F4C30  PUSH 0x19
006F4C34  CALL dword ptr [EDX + 0x278]
006F4C3C  PUSH EDI
006F4C3D  PUSH 0x3
006F4C41  CALL dword ptr [EAX + 0x278]
```

**Evidence:** `TechnoClass__Receive_Radio`, `RadioClass__Transmit_Radio_Impl`, and `RadioClass__Receive_Radio` decompiles; `get_assembly_context` at `0x006F4C34` and `0x006F4C41`.

**Active in YR:** Conditional. The sender/receiver code is active. Whether the ordinary post-unload cleanup fires on a particular replay frame depends on later unit state and remaining contact state.

### 3.8 Relationship to `0x16` and `0x15`

After the building's directed `0x18`, the building sends `0x16`. `UnitClass__Receive_Radio(0x16)`:

- first calls `FootClass__Receive_Radio`;
- may call locomotor vtable `+0x4C(0x4000)` and return `1` without `0x15`;
- on later/aligned passes, requires not-moving, destination non-null, `Unit+0x418 != 0`, destination `WhatAmI()==6`, and current mission `7`, then sends directed `0x15` to the destination.

Key assembly:

```text
007376BA  CALL 0x004D8FB0        ; FootClass receive first
007376BF  MOV  AL, byte ptr [ESI + 0x6AF]
00737709  CALL dword ptr [EDX + 0x4C]   ; locomotor turn/sync
0073774A  MOV  AL, byte ptr [ESI + 0x418]
0073775C  CALL dword ptr [EDX + 0x2C]   ; destination WhatAmI
0073776E  CMP  EAX,0x7                  ; mission 7
00737776  PUSH 0x15
0073777A  CALL dword ptr [EDX + 0x278]
```

The building receiver for stock refinery `0x15` is separate: `BuildingClass__Receive_Radio(0x15)` checks `BuildingType+0x16B3` and queues sender mission `0x10` with argument `0`, then returns `1`.

**Evidence:** `get_assembly_context` at `0x007376AD`, `0x007376BF`, `0x00737705`, `0x0073774A`, `0x00737776`; prior `BUILDING_RECEIVE_RADIO_0X15_DOCKUNLOAD_HANDOFF_GHIDRA_REPORT.md` for `0x0043C788..0x0043C7A0`.

**Active in YR:** Yes. This is the stock `Mission_Enter -> Building 0x0E -> 0x18 -> 0x16 -> 0x15 -> Mission 0x10` path.

### 3.9 DockUnload exit does not clear `+0x418`

`UnitClass__Mission_Deploy_Building @ 0x0073D630` state-4 exit clears `Unit+0x6D1`, not `+0x418`:

```text
0073E1F6  MOV byte ptr [ESI + 0x6D1],0x0
```

No `+0x418` read or write appears in that state-4 exit block. `+0x418` is therefore not the DockUnload active latch.

**Evidence:** `get_assembly_context` at `0x0073E1F6`; prior `UNITCLASS_0X418_DOCK_FLAG_LIFECYCLE_AND_CONSUMERS_GHIDRA_REPORT.md`.

**Active in YR:** Yes for stock refinery unload completion.

## 4. Timing / Reply-Code Timeline

| Step | Sender -> Receiver | Message/reply | Mutations | Return observed by caller |
|---:|---|---|---|---|
| 1 | Unit -> Building | `0x02 HELLO` | `Contacts[]` insert if accepted; no `+0x418` | `1` accepted, `10` denied, or idempotent `1` if already present |
| 2 | Unit -> Building | `0x0E CAN_DOCK` | Building may refresh/contact-check; no `+0x418` from `0x0E` itself | case continues only on receiver gates |
| 3 | Building -> Unit | `0x12 MOVE_TO_CELL` | Unit may set destination/move; if already at cell, no move | `0x14` only if already at accepted cell; otherwise `1` |
| 4 | Building -> Unit | directed `0x18` | Unit `+0x418=1`, then propagation sets Building `+0x418=1`; no `Contacts[]` mutation | ignored by building |
| 5 | Building -> Unit | directed `0x16` | First ordinary call can turn-sync; later call can send `0x15` if `+0x418` and mission/destination gates pass | building checks only this return; `1` suppresses fallback event |
| 6 | Unit -> Building | `0x15 DOCK_NOW` | Stock DockUnload building queues sender mission `0x10, queued=0`; no `+0x418` clear | `1` |
| 7 | later cleanup | `0x19` / `0x03` / `0x08` depending path | `0x19` clears `+0x418`; `0x03` removes `Contacts[]` | conditional |

`FootClass__Mission_Enter @ 0x004D9290` is the repeat driver for later attempts: it sends `0x0E` through vtable `+0x278` at `0x004D92B2..0x004D92B9`, checks `EAX == 1`, then schedules the next enter retry from `[Enter] Rate=.016` plus `RandomRanged(0,2)`. The stock delay is `ftol(.016 * 900) + RandomRanged(0,2)`, i.e. `14..16` ticks.

**Evidence:** decompile `FootClass__Mission_Enter`; assembly context `0x004D92B2`; `ini/rulesmd.ini:[Enter] Rate=.016`.

## 5. Implementation Handoff

| Verified behavior | Evidence | Current Rust surface | Required implementation effect | Acceptance scenario / test suggestion |
|---|---|---|---|---|
| `Contacts[]` is owned by `0x02`/`0x03`, not by `0x18` | `RadioClass__Transmit_Radio_Impl`; `0x0065A970`; `RadioClass__Receive_Radio` | `src/sim/miner/miner_dock.rs::RefineryDockContacts.contacts` | Keep contact admission/removal separate from `contact_entered`; do not insert contacts when modeling `0x18` | `radio_0x18_does_not_create_refinery_contact_slot` |
| Building sends `0x18` only after `0x12` returns `0x14` | `0x0043CAB4..0x0043CAC1`; `0x0043CAC9..0x0043CACE` | `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter` / `phase_awaiting_accepted_cell` | Mark contact-entered only on already-at-accepted-cell handoff, not on initial movement order | `accepted_cell_movement_reply_1_does_not_mark_contact_entered` |
| Building ignores `0x18` return and always proceeds to `0x16` after `0x12 == 0x14` | `0x0043CACE` immediately followed by `0x0043CAD4`; compare only at `0x0043CAE1` after `0x16` | same | Do not branch/refuse because a repeated/already-set `0x18` would return `0` through fallback | `repeated_0x18_return_zero_does_not_block_0x16` |
| `0x18` sets mirrored endpoint byte `+0x418`, not `+0x2E4` or pad occupancy | `0x006F4B72`; propagation via `0x006F4B79` | `RefineryDockContacts.contact_entered`; `on_pad` | Preserve contact-entered as separate from `on_pad`; no reciprocal link write | `radio_0x18_sets_contact_entered_without_on_pad_or_link` |
| First-set `0x18` returns before base RadioClass; already-set falls through to base/Object and returns `0` | `0x006F4B61`, `0x006F4B81..0x006F4B8A`; `ObjectClass__Receive_Radio` | any future radio-history/debug state | If radio history is modeled for determinism, do not log first-set `0x18` as a base receive; already-set fallback may log | `first_set_0x18_bypasses_radio_history` |
| `0x19` clears mirrored byte but does not itself remove `Contacts[]` | `0x006F4BA6..0x006F4BAD`; RadioClass special cases only `0x02`/`0x03` | `clear_contact_entered` vs `release_contact` | Keep `contact_entered` clear separate from contact-slot release, unless the path also sends `0x03` | `radio_0x19_clears_contact_entered_without_contact_slot_release` |
| `0x08` and `0x03` bridge byte clear and contact release | `0x006F4C34..0x006F4C41`; RadioClass `0x03` paths | `release_contact`, `cancel_miner`, post-unload cleanup | Model `0x19` before/beside `0x03` where the binary does; do not let break skip the byte clear | `radio_break_clears_entered_flag_before_contact_release_effects` |
| DockUnload state-4 clears `+0x6D1`, not `+0x418` | `0x0073E1F6` | `phase_unloading` / `Departing` | Do not clear `contact_entered` solely because cargo became empty; clear only through cleanup/break equivalent | `dock_unload_state4_does_not_clear_contact_entered_directly` |

Local Rust scan notes: current surfaces already distinguish `contacts`, `contact_entered`, and `on_pad` in `src/sim/miner/miner_dock.rs`, and current phases include `FaceSync` and `MissionQueued` in `src/sim/miner/mod.rs`. Future implementation work should preserve that split and audit any remaining cleanup/release behavior against the `0x19`/`0x03` ordering above.

## 6. Negative Facts / Do Not Do

- Do not model `0x18` as `HELLO`; `HELLO(0x02)` owns contact insertion.
- Do not mutate `Contacts[]` for `0x18` or `0x19`; only `0x02` and `0x03` have RadioClass contact-array writes.
- Do not gate building `0x16` on the return value of `0x18`; the binary ignores `0x18` return.
- Do not treat an already-set `0x18` fallback return `0` as docking failure.
- Do not write `unit+0x2E4` or `building+0x2E4` for `0x18`.
- Do not equate `+0x418` with `+0x6D1`; DockUnload state-4 clears `+0x6D1` only.
- Do not start unload, play deploy sound, set pad occupancy, snap position, or queue mission `0x10` on `0x18`.
- Do not clear contact-entered state merely because the unload FSM emptied cargo; require a modeled `0x19`/break/cleanup equivalent.
- Do not collapse `0x16 -> 0x15` and per-cell adjacent-building `0x15` into the `0x18` event; they are later consumers of the flag.
- Do not rely on `QueueingCell` or `DockingOffset%d` for this `0x18` lifecycle; they are outside the verified `0x18` sender gate.

## 7. Remaining Uncertainty

- Exact replay frame for the ordinary post-unload `0x08 -> 0x19 -> 0x03` cleanup cascade remains a runtime-trace question. Static code proves the possible path and gates, but not the exact live-frame after every normal unload.
- The global non-dock semantic name of `Techno+0x418` remains broader than this slice. This report only names the field by its verified refinery/dock contact role.
- Building contact-capacity growth from `NumberOfDocks` was not freshly re-decompiled in this slot. Existing `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` verifies that building constructors grow `Contacts[]`; for stock `GAREFN/NAREFN`, local INI shows `NumberOfDocks=1`, so this does not block the stock refinery handoff.

## 8. Open Questions - Final State

- `[RESOLVED] OQ-18C-001 - Does BuildingClass itself handle message 0x18? -> No. BuildingClass switch cases are `3,8,0xB,0xC,0xD,0xE,0xF,0x10,0x15`; building-side `0x18` falls through to TechnoClass.`
- `[RESOLVED] OQ-18C-002 - Does 0x18 write Contacts[]? -> No. `RadioClass__Transmit_Radio_Impl` has contact-array mutation only for `0x02` and `0x03`; generic messages directly dispatch.`
- `[RESOLVED] OQ-18C-003 - What field does 0x18 write? -> Byte `Techno+0x418 = 1` at `0x006F4B72`.`
- `[RESOLVED] OQ-18C-004 - Does 0x18 also affect the building endpoint? -> Yes by propagation: miner sets and transmits `0x18` back; building falls to TechnoClass and sets its own `+0x418`.`
- `[RESOLVED] OQ-18C-005 - What stops infinite 0x18 ping-pong? -> The already-set endpoint falls through to RadioClass/ObjectClass and returns `0` instead of rewriting/repropagating.`
- `[RESOLVED] OQ-18C-006 - Does Building 0x0E require the 0x18 return to be 1? -> No. It ignores the 0x18 return and compares only the later 0x16 return.`
- `[RESOLVED] OQ-18C-007 - What clears the 0x18 state? -> `0x19` clears `+0x418` when nonzero and propagates; `0x03`/`0x08` paths can bridge this with contact release.`
- `[RESOLVED] OQ-18C-008 - Does DockUnload exit clear it? -> No. State-4 clears `+0x6D1`, not `+0x418`.`
- `[DEFERRED] OQ-18C-009 - Exact runtime post-unload cleanup frame?` Category: needs runtime trace.
- `[DEFERRED] OQ-18C-010 - Full non-dock meaning of `+0x418`?` Category: out of scope.

## Sources

- Ghidra read-only decompile: `BuildingClass__Receive_Radio`.
- Ghidra read-only decompile: `TechnoClass__Receive_Radio`.
- Ghidra read-only decompile: `RadioClass__Transmit_Radio_Impl`.
- Ghidra read-only decompile: `RadioClass__Receive_Radio`.
- Ghidra read-only decompile: `ObjectClass__Receive_Radio`.
- Ghidra read-only decompile: `FootClass__Mission_Enter`.
- Ghidra assembly contexts: `0x0043CAAE`, `0x0043CAC7`, `0x0043CAD4`, `0x006F4B72`, `0x006F4B79`, `0x006F4BA6`, `0x006F4BAD`, `0x006F4C34`, `0x006F4C41`, `0x007376AD`, `0x007376BF`, `0x00737705`, `0x0073774A`, `0x00737776`, `0x0073A558`, `0x0073A5C3`, `0x0073A936`, `0x0073E1F6`, `0x004D92B2`.
- Existing context reports: `RADIO_0X18_CONTACT_FLAG_LIFECYCLE_GHIDRA_REPORT.md`, `UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_SCHEDULING_GHIDRA_REPORT.md`, `UNITCLASS_0X418_DOCK_FLAG_LIFECYCLE_AND_CONSUMERS_GHIDRA_REPORT.md`, `BUILDING_RECEIVE_RADIO_0X15_DOCKUNLOAD_HANDOFF_GHIDRA_REPORT.md`, `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`.
- INI evidence: `ini/rulesmd.ini`, especially `[CMIN]`, `[HARV]`, `[GAREFN]`, `[NAREFN]`, and `[Enter]`.
- Rust surface scan only, no edits: `src/sim/miner/mod.rs`, `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_tests.rs`.
