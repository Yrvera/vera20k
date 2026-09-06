# Phase 7 disparity scan — GSI-07.37 Radio contact/link protocol + GSI-07.38 Docking reservations, queues, authority handoff

Date: 2026-09-05. Read-only. Program: `gamemd.exe` (Ghidra `testProsjekt`, image base 0x400000, 10073 functions).
Rust at `feature/phase-7-parity-close-284594` (worktree `clean-slate-system-impl-891469`), HEAD 4b89ef52.

Evidence labels: **[bin]** = decompile/disassembly/memory read in this scan; **[bin-prior]** = prior report, spot-checked here where marked; **[rust]** = file:line read in this scan; **[ini]** = retail `ini/*.ini` read in this scan.

## 0. Verdict on the recorded System Map status

- **GSI-07.37 (recorded `rust_implementation: PARTIAL`, `parity: DRIFT`) — CONFIRMED.** The Rust bus (`src/sim/radio/`) reproduces the RadioClass primitives (HELLO/BREAK slot bookkeeping, sparse capacity slots, broadcast-BREAK on limbo) but (a) is a *shadow* of a separate registry (`ProductionState.dock_reservations: RefineryDockContacts`) that still makes every admission decision, (b) represents only the Structure receiver class (no Foot/Unit/Aircraft class effects), (c) models the `+0x418` dock-entered flag one-sided, (d) sizes building contact capacity lazily at first HELLO instead of at construction, and (e) sends the post-mortem BREAK unconditionally where native `Destroy(false)` BREAKs non-ally contacts only. None of (a)–(e) produces a demonstrated player-visible difference in a stock skirmish refinery cycle; they are structural drift with dormant reach (§2).
- **GSI-07.38 (recorded `native_evidence: UNCHECKED`, `parity: UNCHECKED`) — now ANCHORED; parity DRIFT.** Native has **no stored dock queue anywhere** (refinery, airfield, repair depot): admission is RadioClass contact-slot capacity + per-unit `Mission_Enter` re-probe every 14..16 frames [bin]. Rust matches this for refineries and airfields but the repair depot (`building_dock.rs`) still runs a FIFO with promotion (demonstrated DRIFT, §2 M12). Movement authority never moves to the building: `0x12 MOVE_TO_CELL` installs a NavCom on the unit and the unit's own locomotor drives [bin]; Rust matches in authority but uses a grid-bypass direct move (§2 M8).

## 1. Scope enumeration (native mechanisms, addresses)

### 1.1 RadioClass primitives [bin]

| Function | Address | Vtable slot | Decisive body facts |
|---|---|---|---|
| `RadioClass::Transmit_Radio_Impl` | 0x0065A970 | +0x27C | target null ⇒ `Contacts[0]`, none ⇒ return 0. **HELLO(2)**: scan own slots; target already present ⇒ return 1 without dispatch; first-null slot remembered; **no null slot ⇒ `Transmit_Radio(3, Contacts[0])` then reuse slot 0**; call `target->Receive_Radio(+0x194)`; reply 1 ⇒ store target, return 1; else return 10. **BREAK(3)**: null *every* own slot equal to target, then dispatch. Other ids: pass-through. |
| `RadioClass::Transmit_Radio` | 0x0065AAA0 | +0x278 | wrapper: `Impl(msg, &g_RadioScratchBuffer, target)` |
| `RadioClass::Transmit_Radio_ToFirst` | 0x0065ACB0 | +0x274 | `Contacts[0]` non-null ⇒ `Impl(msg, scratch, Contacts[0])`, else 0 |
| `RadioClass::Broadcast_Radio_ToAll` | 0x0065ACE0 | +0x280 | ascending slot walk `0..Capacity`, re-reads each slot, non-null ⇒ `Impl(msg, scratch, slot)` |
| `RadioClass::Receive_Radio` | 0x0065A820 | +0x194 (RadioClass vtable 0x007F0508+0x194 = 0x007F069C → `20 A8 65 00` [bin read_memory]) | RadioHistory shift (+0xD4/+0xD8/+0xDC). **BREAK**: find sender slot ⇒ `ObjectClass::Receive_Radio(sender,3)`, null slot, return 1; not found ⇒ fall to ObjectClass. **HELLO** (requires Owner +0x6C ≠ 0): `Is_Ally_ByObject` gate (×2) ⇒ 10; sender already present ⇒ 1; first-null insert ⇒ 1; full ⇒ 10 (**no receiver-side eviction**). Others ⇒ `ObjectClass::Receive_Radio`. |
| `ObjectClass::Receive_Radio` | 0x005F5320 | — | 0x0D ⇒ `vtable+0x124(2)` (anim reset), return 1; 0x22 ⇒ health ratio ≥ `Rules+0x16F8` ⇒ 10 else 1; everything else ⇒ **0**. |
| `RadioClass::Contact_With_Whom(i)` | 0x0065AD30 | — | `Contacts[i]` |
| `RadioClass::FindDockSlot(obj)` | 0x0065AD90 | — | first index equal to obj, else −1 |
| `FUN_0065ADF0` (free-or-present probe) | 0x0065ADF0 | — | true if any slot is null **or** equals arg |
| **`0x0065AE30` — MISLABELED `PathType__Has_Valid_Steps`** | 0x0065AE30 | — | body: loop `Contacts[0..Capacity]`, return true if any non-null. It is `RadioClass::In_Radio_Contact()`. Reached from `Mission_Deploy_Building` state 4 (0x0073E26A), `UnitClass::Receive_Radio` case 0x0E, `FootClass::Receive_Radio` case 0x17. ~30 research docs and several Rust comments carry the wrong label (list via `grep -rl Has_Valid_Steps docs/research`). |
| `RadioClass::Set_Contact_Count(n)` | 0x0065AE60 | — | grow-only vector resize, new slots zeroed |
| `RadioClass::PointerExpired` | 0x0065AAC0 | +0x28 | nulls every slot equal to the expired pointer when control flag set — **no BREAK sent** |
| `TechnoClass::Limbo_Tail_CallConceal` | 0x0065AA80 | +0xD4 (Radio vtable) | `!InLimbo ⇒ vtable+0x280(3)` (broadcast BREAK) then `ObjectClass::Limbo/Conceal` |
| `BuildingClass::Constructor` | 0x0043B740 | — | 0x0043BCBD reads `Type+0x1780` (NumberOfDocks), clamps `<1 ⇒ 1` (0x0043BCC3..CC8), `Set_Contact_Count` at 0x0043BCD0 [bin disassembly] |
| `RadioClass::Constructor` | 0x0065A750 | — | one slot, Capacity 1 (prior report; consistent with `Contacts::default()` in Rust) |

### 1.2 Receive_Radio overrides and dispatch order [bin]

Vtable +0x194 reads: Techno 0x007F4960+0x194 = 0x007F4AF4 → `TechnoClass::Receive_Radio` (data xref confirmed); Building 0x007E3EBC+0x194 = 0x007E4050 → `D0 C2 43 00` = 0x0043C2D0 [bin read_memory]; Unit vtable slot 0x007F5E04 → 0x00737430; Aircraft 0x007E2438 → 0x004190B0; FootClass override 0x004D8FB0 sits in two vtables (0x007E8E28, 0x007EB1EC = Foot and Infantry; Infantry has no override).

Dispatch chain (each override handles its cases then calls the parent for the rest, so the parent tail runs *after* the child's effects):

- `BuildingClass::Receive_Radio @ 0x0043C2D0` → `TechnoClass::Receive_Radio @ 0x006F4AB0` → `RadioClass::Receive_Radio` → `ObjectClass::Receive_Radio`.
- `UnitClass::Receive_Radio @ 0x00737430` → `FootClass::Receive_Radio @ 0x004D8FB0` → Techno → Radio → Object.
- `AircraftClass::Receive_Radio @ 0x004190B0` → Foot → Techno → Radio → Object (with a leading deaf latch: missions 4/0x1A/0x1B/0x1E/0x1F with `+0x294 == 0` ⇒ return 0).
- Infantry: FootClass override directly.

Cases handled per level (stock-reachable ones bolded):

| Level | Cases |
|---|---|
| Techno 0x006F4AB0 | **3** (both sides `+0x418` set ⇒ send 0x19 to sender, then Radio), **7/9/0x16** (send 0x18 back to sender, then Radio, return 1), **8** (send 0x19 then BREAK to sender), **0x18** (Aircraft w/ `Type+0xE0D` skip; else if `+0x418==0` set it and echo 0x18 back), **0x19** (if `+0x418` set clear it and echo 0x19 back), 0x1A/0x1B (`+0x419` set/clear, echo), **0x1C** (repair tick: ratio ≥ `Rules+0x16F8` ⇒ 10; else cost via `Type vtable+0xB0/+0xB4`, step≥1, spend, heal, 0x21 on full else 1; 0x20 if unaffordable), 0x1E, 0x1F |
| Foot 0x004D8FB0 | 0x11, **0x12** (payload cell == current cell ⇒ **0x14** without any write; else mission Guard(5)+no queued ⇒ Queue Move(2); queued Enter(7) & Ready ⇒ Commence; `Set_Destination(cell,1)` (+0x480); `+0xC8=frame, +0xCC=junk, +0xD0=0`; return 1), **0x13** (`*payload=NavCom`; NavCom≠0 & locomotor `Is_Moving` ⇒ 10 else 1), **0x17** (In_Radio_Contact & NavCom==Contacts[0] ⇒ Set_Destination(0); mission Sleep ⇒ Queue Guard+Commence; mission Enter ⇒ Queue Guard; `!+0x6AF && NavCom==0` ⇒ Scatter), 0x1C (NavCom≠0 ⇒ 10, else Techno), 0x23 |
| Unit 0x00737430 | **3** (mission Return(0xC) ⇒ Queue Guard, then Foot, return 1), 7, 0xE (unit-as-transport), 0xF, **0x15** (return 5; sound if passengers==max), **0x16** (Foot first; `!+0x6AF` & facing≠0x4000 ⇒ locomotor `Do_Turn(0x4000)` return 1; else not moving & `+0x418` & Contacts[0] is Building(6) & mission Enter ⇒ send 0x15 to Contacts[0]; return 1), **0x17** (harvester/weeder & `+0x6D1` ⇒ clear, Scatter, Queue Harvest(10)+Commence), 0x24 |
| Building 0x0043C2D0 | **3** (`GrandOpening(1)` at 0x0043CD01..CD05 [bin], then Techno, return 1), **8**, 0xB, 0xC, 0xD, **0xE**, 0xF, 0x10, **0x15**; everything else Techno |
| Aircraft 0x004190B0 | 8, 0xE, 0xF, 0x12, 0x13, 0x15, 0x17, 0x1D, 0x1F, 0x21 |

### 1.3 Message ids as sent in active YR (sender → receiver) [bin unless noted]

| Id | Rust name | Sender site | Receiver effect |
|---|---|---|---|
| 0x01 | Roger (reply) | — | wire string verified (prior plate comment on 0x0065A970: `RADIO_ROGER/HELLO/NEED_TO_MOVE/WANT_RIDE` from `Carryall_Move` debug strings — [bin-prior], not re-read) |
| 0x02 | Hello | `Mission_Harvest` state 2 (0x0073EE51), `Building 0x0E` (building→unit), `Unit 0x0E`/`0x07` | link both ends |
| 0x03 | Break | `Mission_Deploy_Building` state 4 (0x0073E279 ToFirst), `Mission_Enter` reject path, `Techno 0x08`, `PerCellProcess` harvester-away tails, Limbo broadcast, `Destroy(false)` non-ally | teardown |
| 0x07/0x09 | DockingComplete (inferred) | not traced this scan | Techno: echo 0x18 |
| 0x08 | RequestClearance | `UnitClass::PerCellProcess` (`+0x418` set, mission ≠ Enter/Unload) | Building: UnitRepair/Bunker near ⇒ 1; Techno sends 0x19+BREAK; factory/repair/bunker ⇒ 0x17 else 1 |
| 0x0E | CanDock | `FootClass::Mission_Enter @ 0x004D9290` every dispatch; `PerCellProcess` 0x17 branch | see M6 |
| 0x0F | CanEnter | PerCellProcess (transport/bio-reactor), `AircraftClass::FindBuildingToDock` | admission query |
| 0x12 | MoveToCell | Building 0x0E (payload = CellClass* of NW+(3,1)); Unit/Aircraft 0x0E; Building 0x0E non-dock path (`Get_CellClass_At_Coord`) | Foot: 0x14 if already there else NavCom |
| 0x13 | NeedToMove | Building 0x0E, Unit 0x0E, Aircraft 0x0E | Foot: moving ⇒ 10 |
| 0x14 | CellAccepted (reply) | Foot 0x12 | — |
| 0x15 | DockNow (inferred; native "pad arrival") | `UnitClass::PerCellProcess @ 0x00739EC0` `ToFirst(0x15)` when mission Enter/0x19 and cell == building's `GetDockCoord` cell; Unit 0x16 (facing reached) | Building DockUnload ⇒ `sender->Queue_Mission(0x10 Unload, 0)` return 1 (asm 0x0043C788..C7A9 [bin-prior], reachable in decompile [bin]) |
| 0x16 | TimingSync (inferred) | Building 0x0E after 0x18 | Unit: turn to 0x4000 / send 0x15 |
| 0x17 | Queued (reply / verb) | Building 0x08 (factory/repair/bunker), Building 0x0E non-dock loop | Foot 0x17 / Unit 0x17 |
| 0x18 | EnterDock | Building 0x0E (after 0x12→0x14), Techno 7/9/0x16 echo, Unit 0x07 | set `+0x418`, echo |
| 0x19 | LeaveDock | Techno 3 (both flags), Techno 8 | clear `+0x418`, echo |
| 0x1A/0x1B/0x1E/0x1F/0x1D/0x21/0x22/0x23/0x24 | see Rust enum | not in refinery scope; senders not traced this scan | — |

Not modelled in Rust and confirmed absent from stock refinery flow: 0x0B/0x0C/0x0D senders (out of lane), 0x10 as a wire message (only a `Queue_Mission` argument).

### 1.4 Rust owners [rust]

- Bus: `src/sim/radio/mod.rs` (`transmit`, `transmit_hello`, `transmit_break`, `broadcast_break`), `contacts.rs` (`Contacts` sparse slots), `receive.rs` (`receive_radio` → Structure only: `refinery_receive`/`bunker_receive`; common BREAK clear).
- Registry mirror (decision authority): `src/sim/miner/miner_dock.rs` (`RefineryDockContacts` — no queue; `DockReservations` — FIFO, used only by repair depot).
- Miner FSM: `src/sim/miner/miner_dock_sequence.rs` (`bus_hello` :199, `bus_enter_dock` :221, `bus_break` :233, `phase_approach` :844, `phase_mission_enter` :922, `phase_face_sync` :1048, `phase_departing` :1333, `interrupt_refinery_docked_miners` :582, `abort_*` :668/:699), `src/sim/miner/miner_system.rs` (`handle_return` :1157, `try_begin_close_return_radio` :1508, `refinery_dock_capacity_for_sid` :1799).
- Generic docking: `src/sim/docking/building_dock.rs` (repair depot FSM), `aircraft_dock.rs` (`AirfieldDocks` first-free-slot, no queue; `tick_aircraft_docks`), `pad_geometry.rs`.
- Lifecycle: `src/sim/world/lifecycle.rs` `techno_limbo_with_context` :1920 (→ `broadcast_break` :1942), `postmortem_exact_zero_callbacks` :2038 (→ `broadcast_break` :2083), `uninit_with_context` :2803 (→ `notify_pointer_expired` :2568 → `techno_limbo_with_context` :2845).
- Consumers of `radio_contacts`: `world/techno_ai.rs:817` (mirage disguise block), `world/techno_ai_cloak.rs:85` (slot-0 WeaponsFactory), `mission/authority.rs:312` (unit-ready slot 0), `world/world_commands.rs:2667` (duplicate Enter no-op), `world_hash.rs:1655`, `snapshot.rs:997`.
- Production reachability: `advance_tick` (`world/mod.rs:6748`) → `sweep_dead_dock_reservations` :6916, miner tick via `techno_ai` object pass (`miner_system::process_miner_with_resource_authority` :584 → `handle_dock_sequence`), `tick_building_docks` :7822, `tick_aircraft_docks` :7828; death → `uninit_with_rules` from `world/mod.rs:2911/7497`, `techno_ai.rs:388`.

## 2. Per-mechanism findings

### M1 — HELLO/BREAK sender/receiver bookkeeping (07.37)

Native [bin]: §1.1 rows 1 and 5. Rust [rust]: `radio/mod.rs::transmit_hello` (already-linked ⇒ Roger without dispatch ✓; on Roger `insert_evicting` ✓ slot-0 self-evict), `transmit_break` (null every matching sender slot ✓ then dispatch ✓), `receive.rs::refinery_hello` (alive gate, owner-equality gate, idempotent, first-null insert, full ⇒ Negatory ✓), common BREAK tail nulls first matching receiver slot ✓ (native nulls the one found slot; Rust `remove` = first match — same for a 1-slot sender).

- **MATCH (bounded)** for: capacity-1 sender, capacity-N receiver, same-owner sender, receiver full ⇒ 10 with no eviction, BREAK order (sender clear → receiver class effect → receiver common clear) — mirrored by tests `lifecycle_authority_break_*` (receive.rs:565..).
- **DRIFT (dormant in stock)**: native sender-full path sends `BREAK(3)` to `Contacts[0]` **before** dispatching HELLO (0x0065AA2A..AA36 per prior report; decompile shows `vtable+0x278(3, Contacts[0])` then `slot=0`). Rust `insert_evicting` silently overwrites after the reply, with no BREAK to the evicted partner. Reach: a miner always `bus_break`s before re-HELLOing (`abort_invalid_refinery`, `phase_departing`, `interrupt_refinery_docked_miners`), so the Rust sender is never full at HELLO time in production. Trigger frequency ≈ 0 in stock; a builder touching multi-contact senders (carryall, transports) will hit it.
- **DRIFT (dormant in stock)**: native receiver HELLO gate is `HouseClass::Is_Ally_ByObject` (allied houses accepted); Rust uses `refinery.owner != miner_owner`. Allied-refinery docking needs the miner's dock finder (`+0x528`) to return an ally's building; stock `Find`-of-own-house makes this unreachable in a 1v1; in team games with shared refineries UNRESOLVED (not traced).
- Native HELLO also requires `this+0x6C` (Owner) ≠ 0 — trivially true.
- **Ordering detail MATCH**: native receiver HELLO on a full list returns 10 *without* touching history-independent state; Rust same.

### M2 — Receive dispatch by class (07.37)

Native [bin]: §1.2. Rust [rust]: `receive_radio` matches only `EntityCategory::Structure` for class effects; Unit/Infantry/Aircraft receivers return `None` and only get the common BREAK clear.

- **MISSING (class effects)**: `UnitClass` case 3 (mission Return ⇒ `Queue_Mission(Guard)`), Techno case 3 `0x19` cascade (needs two-sided `+0x418`, see M5), Techno 7/9/0x16 echo-0x18, Foot 0x12/0x13/0x17, Unit 0x16/0x17. All of these are replaced by direct FSM choreography in `miner_dock_sequence.rs`, which is why the recorded parity is DRIFT rather than MISSING at the *behavior* level. Player-visible effect in the stock dock cycle: none demonstrated — the FSM reproduces the observable sequence (see M6/M9). The residual risk is duplicate authority: a second sender of the same ids (bunker, war-factory exit, aircraft) cannot reuse the receivers.
- Rust `refinery_receive` treats `MoveToCell/TimingSync/DockNow` as inert `Roger` and `CanDock` as unconditional `CellAccepted` — explicitly documented as transitional (receive.rs:120..).

### M3 — Contact capacity (07.37/07.38)

Native [bin]: `BuildingClass::Constructor` sets capacity `max(NumberOfDocks,1)` at construction (0x0043BCBD..BCD0). Retail [ini]: GAREFN/NAREFN `NumberOfDocks=1` (rulesmd.ini:11729, 12521), GAAIRC/AMRADR `=4` (11838, 12358), GADEPT/NADEPT `=1`. Multi-pad: `GetDockCoord @ 0x00447B20` uses `FindDockSlot` index as the pad index [bin-prior, DOCKING_QUEUE_EXIT_REFERENCE_POINTS §3.3, not re-read].

Rust [rust]: `GameEntity::radio_contacts` defaults to 1 slot (`game_entity.rs:1169`); the only production `set_capacity` is `miner_dock_sequence.rs:207` inside `bus_hello` (grow-only, at first HELLO). No spawn-time sizing for any building (`grep set_capacity` → only that site + tests).

- **DRIFT (structural, no stock effect for refineries)**: capacity is 1 until the first HELLO; identical outcome for `NumberOfDocks=1`. For airfields the building's `radio_contacts` is never sized or populated; `AirfieldDocks` (`aircraft_dock.rs:100..`) is a parallel first-free-slot store keyed by NumberOfDocks (`aircraft/mod.rs:532`, `aircraft_dock.rs:510`). Admission outcome and pad index match native's slot semantics (bounded to: first-null scan, idempotent, no queue, release empties the slot). Risk: `radio_contacts`-reading consumers (mirage/cloak/ready/duplicate-enter) never see airfield links; none of them applies to aircraft today.

### M4 — Contact lifetime: Limbo, Destroy, PointerExpired, save/load (07.37)

Native [bin]:
- Limbo: `FootClass::Limbo`/`BuildingClass::Limbo` → `TechnoClass::Limbo_Helper @ 0x006F6AC0` → 0x0065AA80 → `Broadcast_Radio_ToAll(3)` before Conceal [bin-prior BROADCAST report §3.1/3.2; 0x0065AA80 and 0x0065ACE0 re-decompiled here].
- `BuildingClass::Destroy(false) @ 0x0044EBF0`: for each contact, **only if `!Is_Ally_ByObject(contact)`** send `Transmit_Radio(3, contact)`; `Destroy(true)` (the sell/uninit variant after factory abandon) does `vtable+0x280(3)` to all. `FootClass::Destroy(false) @ 0x004D9720`: `Contacts[0]` non-ally ⇒ `ToFirst(3)`; `Destroy(true)` ⇒ team remove + `ToFirst(3)`.
- `ObjectClass::Destroy @ 0x005F5280` → `DispatchPointerExpiredCleanup @ 0x007258D0` → every object's `RadioClass::PointerExpired @ 0x0065AAC0` nulls slots equal to the dying object **silently** (no BREAK, no 0x19). The later Limbo broadcast still finds the dying object's *own* slots intact, so the partner receives BREAK; its Radio tail then finds no matching slot and falls to `ObjectClass::Receive_Radio` (return 0) — but the Techno 0x19 cascade (both `+0x418` set) still runs first.
- Save/load: `RadioClass::Save/Load @ 0x0065AC40/0x0065AB80` serialize contacts only; RadioHistory is self-maintained and had no consumer in the binary-wide scan [bin-prior RADIOHISTORY report; not re-verified].

Rust [rust]: `techno_limbo_with_context` → `broadcast_break` ✓ (ascending slot re-read ✓, test `lifecycle_authority_limbo_break_uses_sparse_slot_order`). `postmortem_exact_zero_callbacks` :2083 → `broadcast_break` **unconditionally** for every category. `notify_pointer_expired` (:2568) does not touch `radio_contacts`. `Contacts` is serde ✓; RadioHistory omitted.

- **DRIFT (no stock effect)**: post-mortem BREAK is unconditional vs native non-ally-only (`Destroy(false)`) — in stock skirmish all contacts are same-house (no cross-house HELLO sender exists in the refinery/airfield/depot flows), so the allied contacts' BREAK simply arrives earlier (post-mortem) instead of at Limbo. Same-tick end state identical. Which `Destroy` variant the exact-zero kill path invokes (`vtable+0xF8` argument) was not traced here — UNRESOLVED; the Rust comment "BuildingClass::Destroy broadcasts BREAK" over-states it (it is ally-filtered for `false`, broadcast for `true`).
- **MATCH (end state)**: PointerExpired silent null is absent in Rust, but the partner's slot is cleared by the subsequent BREAK's common tail; the only difference is that native's partner-side `Radio::Receive_Radio(BREAK)` returns via ObjectClass (0) instead of 1 — no consumer.
- Adjacent, out of lane: `Simulation::clear_radio_contacts_for` (`entity_store.rs:80`, used by `aircraft/drop_payload.rs`) clears **all** slots of the target and every peer slot pointing at it — a non-native mass clear; harmless while senders hold ≤1 slot.

### M5 — `+0x418` dock-entered flag reciprocity (07.37)

Native [bin]: Techno 0x18 sets receiver `+0x418=1` and echoes 0x18 to the sender (so building→miner 0x18 ends with **both** flags set; the echo's echo terminates on the already-set flag). Techno 0x19 clears + echoes. Techno BREAK: `[ESI+0x418] && [EDI+0x418]` (0x006F4C50..4C66 [bin disassembly]) ⇒ 0x19 to sender before Radio tail. Consumers: `Mission_Enter` preserve gate (`(char)param_1[0x106]` = `+0x418`), `UnitClass 0x16` (0x15 only when set), `UnitClass 0x07`, `PerCellProcess` 0x08 cleanup (`+0x418 && mission ∉ {Enter, Unload}` ⇒ `ToFirst(0x08)`), `PerCellProcess` 0x15 re-send when adjacent-north building matches.

Rust [rust]: `GameEntity::dock_entered_with: Option<u64>` on the **unit only**; `refinery_receive(EnterDock)` sets it, `LeaveDock`/`Break` clear it; registry mirror `contact_entered` (`miner_dock.rs`). `war_factory_exit.rs` reuses the same flag for factory exit.

- **DRIFT (represented as PARTIAL; no demonstrated stock effect)**: building-side flag absent; the 0x19 cascade is replaced by a direct clear in `refinery_break_effect`. Stock end state after a normal cycle is identical (both flags 0, both contact lists empty). Divergence would surface only for a consumer of the *building's* flag — none was found in this scan's decompiles (Building 0x0E/0x15/0x08 read the sender's flag or none).

### M6 — Refinery admission chain and its timing (07.38 → 07.37)

Native [bin], stock `DockUnload=yes` refinery, `HARV/CMIN`:

1. `Mission_Harvest` state 2 (0x0073E5E0): only when **`NavCom == 0`** (a moving miner never HELLOs: `if (pNavCom != 0) goto rate-delay`), distance ≤ `HarvesterTooFarDistance*256` (`Rules+0xD78`, =5 cells [ini rulesmd.ini:293]) or `ChronoHarvTooFarDistance` (`+0xD7C`, =50 [ini :294]) ⇒ `Transmit_Radio(2, refinery)`; reply 1 ⇒ state 3. Any other outcome: re-find dock (editor-mode counter bump), if distance > 0x300 leptons (3 cells) **or Teleporter** ⇒ NavCom = `Find_Nearby_Passable_Cell(NW + Type+0x1618/0x161C (QueueingCell=4,1 [ini artmd]))`. Epilogue: `ftol(Rate*900) + RandomRanged(0,2)` (14..16 stock).
2. State 3: `Queue_Mission(7 Enter, 0)`, return 1.
3. `FootClass::Mission_Enter @ 0x004D9290`, every dispatch: target = `Contacts[0]` else filtered `+0x218`; **`Transmit_Radio(0x0E, target)`**; reply 1 **or** `+0x418` set ⇒ stay (consume queued destination if NavCom empty; Teleporter re-issues `Set_Destination(old NavCom,1)`); else `ToFirst(3)` + `vtable+0x484(0,1)`. Epilogue 14..16 (Enter Rate 0.016 [bin-prior]).
4. `BuildingClass::Receive_Radio(0x0E)`: Techno (no-op) → `+0x660` gate (M10) → not UnitRepair/Bunker → not Hospital/Armory → `!Contains(sender) && free-or-present ⇒ Transmit(2, sender)` (building→miner HELLO; miner's Radio tail returns 1 immediately if it already holds the building) → `Contains && DockUnload ⇒ GetDockCoord-side check` (result unused) → `Transmit(0x13, sender)`; reply ≠1 (unit still moving) ⇒ return 1 → payload = `Get_CellClass(NW+(3,1))` → `Impl(0x12, &cell, sender)`; reply ≠ 0x14 ⇒ return 1 (this is the *move assignment*) → `Transmit(0x18)` → `Transmit(0x16)`; reply ≠1 ⇒ `sender->Scatter(DAT_0089C848,1,1)`; return 1. **A full refinery returns 1 with no write** (no queue).
5. Miner at accepted cell: Unit 0x16 ⇒ `Do_Turn(0x4000)` per dispatch until facing east, then 0x15 ⇒ building `Queue_Mission(0x10 Unload,0)` on the miner → `Mission_Deploy_Building`.
6. **Timing correction (new)**: `MissionClass::Mission_Dispatch @ 0x005B3060` writes `nDispatchStartFrame = frame` and `nDispatchDelayFrames = handler return` **after** the handler returns. Foot 0x12's `+0xC8/+0xCC/+0xD0` writes therefore never survive a Mission_Enter dispatch; the "immediate re-probe after 0x12" implied by RADIO_0X12 report §3 does not occur on the refinery path. The next 0x0E is 14..16 frames later.

Rust [rust]: `phase_approach` HELLO on `approach_hello_timer` (Harvest cadence) ✓; accepted ⇒ `schedule_enter_retry` (14..16, one `RandomRanged(0,2)` on `scenario_rng`) ✓ then `phase_mission_enter` re-HELLOs every due window and, at the accepted cell, `bus_enter_dock` + `FaceSync`; `phase_face_sync` sends `EnterDock` once per due window (L20) and hands off with one RNG draw (L9) ✓; `refinery_can_dock_queue_cell` = NW+(3,1) ✓ (`REFINERY_ACCEPTED_DX/DY`).

- **MATCH (bounded)**: accepted cell (3,1); 14..16 cadence for Enter and Harvest; one RNG draw per dispatch; no stored queue; full ⇒ stay in Enter and re-probe.
- **UNRESOLVED (±1–2 frames)**: native inserts the state-3 hop (Queue Enter, return 1) and a `Commence` before the first 0x0E; Rust arms the 14..16 retry directly at ROGER. Whether `Commence` zeroes the dispatch timer (immediate first 0x0E next frame) was not read (`MissionClass::Commence` not decompiled). Rust's `mission_commence_exact` exists (`mission/authority.rs:620`) but the miner FSM does not route the HELLO→Enter hop through it.
- **DRIFT (CMIN only, demonstrated at code level)**: Rust `phase_approach` HELLOs while `movement_target.is_some()`; native never HELLOs with a NavCom set. Visible when the pad frees while the chrono miner is driving to the staging cell: Rust turns toward the pad from mid-route; native first reaches the staging cell. Frequency: every chrono-miner wait with a slot freeing mid-drive (a few times per game with 2+ CMIN per refinery). HARV is protected by `handle_return`'s adjacency gate (it enters `Dock` only near the queue cell).
- **DRIFT (HARV, minor)**: native close-HELLO radius is 5 cells from the refinery object (any stopped position); Rust HARV requires adjacency to the queue cell (`is_adjacent_or_at`/`close_enough`) — `try_begin_close_return_radio` is chrono-only (comment at miner_system.rs:1516). Visible only when a HARV stops 2–5 cells away without reaching the queue cell (blocked); it then keeps re-pathing instead of linking.

### M7 — Where a second miner waits, and who docks next (07.38)

Native [bin]: no building-side queue (M6 step 4). The rejected miner's NavCom becomes `Find_Nearby_Passable_Cell(seed = NW+QueueingCell)` (Mission_Harvest state 2 far branch; args `(…,2,-1,0,0,1,1,0,0,0,1,&out,0,0)` — whether unit-occupied cells count as impassable was **not** verified). It re-HELLOs each Harvest dispatch once stopped; whoever's dispatch lands first after the slot frees wins (emergent). `NEXT_DOCKER_SELECTION` report agrees [bin-prior].

Rust [rust]: `RefineryDockContacts` has no queue ✓ (`release()` never promotes; test `release_contact_does_not_promote_waiter`). Waiting target = `refinery_queue_cell` = `(rx+4, ry+1)` exactly (`miner_dock_sequence.rs:281`); every waiting miner paths to the same cell and stops adjacent (`issue_move_if_idle` + `is_adjacent_or_at`).

- **MATCH**: no FIFO; re-probe cadence; slot goes to the first re-prober.
- **UNRESOLVED → likely DRIFT**: staging spread for 3+ waiting miners (native nearby-passable search from the seed vs Rust fixed cell + adjacency). Needs `Find_Nearby_Passable_Cell` flag semantics (0x004D....; not read). Frequency: common mid-game (3–4 miners per refinery). Effect: cluster shape at the refinery's east side.

### M8 — Movement authority during docking (07.38)

Native [bin]: the building never drives the unit. `0x12` → `FootClass::Set_Destination(cell,1)` (+0x480) on the unit; the unit's own locomotor moves; `0x16` → unit's locomotor `Do_Turn`; `0x15` → building only *queues a mission* on the unit. State-4 exit: unit `Queue_Mission(Harvest)` + `Commence` (0x0073E24D..E283 [bin disassembly]); `In_Radio_Contact ⇒ ToFirst(BREAK)`; BREAK is **unconditional on a live contact** (the "Has_Valid_Steps" gate in BUILDING_RECEIVE_RADIO_DOCK_CLEARANCE §3.5 is the mislabel from §1.1). `ReleaseDockedHarvester @ 0x004595C0` (Force_Track 0x47) is reachable only through the `+0x2E4` reciprocal link, which stock 0x15 never writes [bin-prior; consistent with 0x15 decompile here].

Rust [rust]: `phase_mission_enter` → `movement::issue_direct_move(…, accepted_cell)` with `bypass_grid = true` (:1004..1020); `phase_departing` → `bus_break` + `mission_queue_exact(Harvest)` + `mission_commence_exact` ✓ and no Force_Track ✓.

- **MATCH (authority)**: unit-owned movement, building only queues missions.
- **UNRESOLVED (path semantics)**: native `Set_Destination` to a footprint cell uses the normal pathfinder over a cell opened by `RemoveOccupy1=3,1` [ini artmd]; Rust bypasses the grid. Visible difference only if the approach from the queue cell to (3,1) would route differently — not compared here (GSI-06 movement lane).

### M9 — Dock slot release and cleanup fallbacks (07.38)

Native [bin]: normal release = state-4 BREAK (M8). Fallbacks in `UnitClass::PerCellProcess @ 0x00739EC0` on every cell crossing: (a) `+0x418 && mission ∉ {Enter(7), Unload(0x10)} && (Contacts[0] ≠ NavCom …)` ⇒ `ToFirst(0x08)` → refinery: Techno 0x19+BREAK, return 1; (b) harvester with `Contacts[0]` a `Refinery=yes` building and mission ∉ {Unload, Enter (current or queued)} ⇒ `ToFirst(3)`. Building side: `Receive_Radio(8)` for DockUnload returns 1 (no 0x17) ✓ [bin].

Rust [rust]: `phase_departing`, `abort_invalid_refinery`, `abort_missing_unload_building`, `interrupt_refinery_docked_miners` (sell path `production_sell.rs:753`) all `bus_break` immediately; a player move-order on a linked miner is handled by the miner FSM abort path (not re-traced here).

- **MATCH (end state)**, timing differs by ≤ one cell crossing in the fallback cases (Rust breaks at the order/abort tick; native at the next `PerCellProcess`). No consumer difference found.
- Sell/capture/destruction: sell → `interrupt_refinery_docked_miners` + uninit (`production_sell.rs:766`) → Limbo broadcast ✓; destruction → post-mortem + Limbo ✓ (M4). Capture (`ChangeOwner`) contact retention: not inspected (prior report `BUILDING_CHANGEOWNER_CONTACT_RETENTION_…` exists; not verified).

### M10 — 0x0E power gate — EXCLUDED for stock refineries

[bin] Case 0x0E reads `[ESI+0x660]` (0x0043C7FB) — the field the decompiler names `HasPower`. Its only clearer is `BuildingClass::GoOffline @ 0x00452360` (`HasPower=false` when drain>0 or `Type+0x1573`), which is player power-toggle / map trigger only [bin + POWER_SYSTEM report §"GoOnline/GoOffline — Player Commands Only"]. GAREFN/NAREFN `TogglePower=no` [ini rulesmd.ini:11757]. Low power does **not** clear `+0x660`. Rust has no dock power gate (`grep has_power src/sim/miner` empty) — correct by exclusion for skirmish; a map-trigger "Disable Power" would diverge (campaign only).

### M11 — Building BREAK side effect `GrandOpening(1)` — UNRESOLVED (low)

[bin] `BuildingClass::Receive_Radio` case 3: `PUSH 1; CALL 0x00447780` then Techno. `GrandOpening(1)` writes `+0x538 = 1` and, only when `+0x534 == -1` or in the editor, re-seeds the stage timer `+0xF8..+0x10C` from `Type+0xF04+stage*0xC`. Consumer of `+0x538` not traced. Rust: `receive.rs` TODO(BLOCKED). No visible effect identified for a refinery.

### M12 — Repair depot docking (07.38 generic) — DRIFT

Native [bin] `BuildingClass::Receive_Radio(0x0E)` non-DockUnload path (UnitRepair): Techno → `+0x660` → `UnitRepair && Contains(sender) && Transmit(0x22, sender)==10 ⇒ 10` → (Bunker gate n/a) → `FUN_0065ADF0` free-or-present ⇒ `Impl(0x12, &Get_CellClass_At_Coord(dock coord), sender)`, return 1; else **for each contact: `Transmit(0x22, contact)`; reply 10 (occupant health ratio ≥ `Rules+0x16F8`, i.e. repaired) ⇒ `Transmit(0x17, contact)`** (Foot 0x17: clear NavCom if it was the depot, Sleep⇒Guard, Enter⇒Guard, Scatter), then free-or-present ⇒ 1 else 10. Waiting units re-probe from their own `Mission_Enter` every 14..16 f (M6 step 3; reply 10 ⇒ BREAK + `+0x484(0,1)` then re-target from `+0x218` next dispatch — the exact re-target source for a player-ordered depot entry was not traced). Repair tick: Techno 0x1C (§1.2). Dock cell: `GetDockCoord` UnitRepair `NumberOfDocks==1 ⇒ building coord + DockingOffset0` [bin-prior]; GADEPT `Foundation=3x3`, no DockingOffset ⇒ center; NADEPT `4x3`, `DockingOffset0=128,0,0` ⇒ origin+(2,1) [ini artmd].

Rust [rust] `building_dock.rs`: `DockReservations` FIFO with promotion on `release`/`cancel`/`cleanup_dead` (`miner_dock.rs:190..`), `WaitForDock` re-calls `try_reserve` each tick (which *enqueues*), `Servicing` heals via `repair_tick` (cost math VERA-internal — Techno 0x1C uses `Type vtable+0xB0/+0xB4` cost/step, not compared here), `ExitDock` releases and does **not** move the unit off the pad (`exit_moves` loop is a no-op, :360..). `depot_dock_cell` = foundation center: GADEPT (rx+1,ry+1) ✓, NADEPT (rx+2,ry+1) ✓ equals `pad_geometry` result.

- **DRIFT (demonstrated)**: (1) FIFO promotion vs native emergent re-probe: with 3 damaged tanks sent together, Rust admits strictly in arrival order; native admits whichever waiter's Enter timer fires first after the pad frees (usually but not always the first arrival). (2) Native evicts a repaired occupant (0x22 ⇒ 10 ⇒ 0x17 scatter) when another unit probes; Rust waits for the occupant's own `RepairComplete` exit and never scatters it. (3) Rust never drives the repaired unit off the pad; native 0x17/Scatter moves it. Frequency: every multi-unit repair session (player-driven, several per game). Player-visible: order and the vacated pad.
- Not in scope, recorded: dock cell for stock depots matches.

### M13 — Airfield admission (07.38 generic) — MATCH (bounded), parallel authority

Native [bin-prior NEXT_DOCKER §Finding 4; AIRFIELD_RADIO_CACHEDDOCK]: `AircraftClass::FindBuildingToDock` re-validates `CachedDock` with 0x0F, admission through 0x0E on the building (helipad path `Type+0x16CB`: `0x12` with `this` payload then `ToFirst(0x18)` [bin, case 0x0E decompile]), pad = contact slot index. Not re-read beyond case 0x0E.

Rust: `AirfieldDocks` first-empty-slot, idempotent, no queue, release empties ✓ (tests `airfield_full_waiter_admitted_by_probe_not_fifo`, `airfield_release_does_not_pin_freed_pad_index`). `tick_aircraft_docks` re-probes every tick (native: per Mission_Enter dispatch cadence — not compared; aircraft lane).

### M14 — Adjacent (out of lane, recorded)

- War-factory exit contact (`production_spawn.rs:308`) is one-sided (unit→factory) plus `dock_entered_with`; native `ExitObject_Main` HELLO links both ends and 0x18 sets both flags. Consumer today: `war_factory_exit.rs` break gate only.
- `Contacts::insert` via `mark_live_contact_with` bypasses the bus in `passenger.rs`, `drop_payload.rs`, `production_spawn.rs` (production) — three writers of the same state besides `transmit`.

## 3. Ranked implementation candidates (grouped by prerequisite)

**Group A — bus becomes the authority (prerequisite for B, D, F)**
1. Retire `RefineryDockContacts` as decision source; `phase_approach`/`phase_mission_enter`/`try_begin_close_return_radio` read `transmit(Hello)` replies; size `radio_contacts` at building spawn from `NumberOfDocks` (`world_spawn`/`production_spawn` building creation) instead of `bus_hello`. Size M (~300 lines net removal). Files: `src/sim/miner/miner_dock.rs`, `miner_dock_sequence.rs`, `miner_system.rs`, `src/sim/radio/receive.rs`, `src/sim/world/world_spawn.rs`, `production_spawn.rs`, `snapshot.rs`, `miner_tests.rs`. Hash impact: `radio_contacts.hash_fold` includes capacity → expect snapshot rebaseline.

**Group B — waiting-miner staging (07.38, visible)**
2. Native HELLO gate `NavCom == 0` for CMIN (`phase_approach` must skip HELLO while `movement_target.is_some()`); HARV 5-cell close-HELLO. Size S. Files: `miner_dock_sequence.rs:844..`, `miner_system.rs:1508..`.
3. `Find_Nearby_Passable_Cell` seeding for the wait cell (needs the native helper's flag semantics — RE task first). Size M. Files: `miner_dock_sequence.rs:281`, `find_nearby_passable_cell_with_index :418`.

**Group C — repair depot re-probe model (07.38, visible)**
4. Replace `DockReservations` FIFO with contact-slot + per-unit Enter cadence re-probe; add 0x22/0x17 occupant eviction and pad exit move; align `repair_tick` cost with Techno 0x1C (`Type vtable+0xB0/+0xB4`). Size M. Files: `src/sim/docking/building_dock.rs`, `miner_dock.rs` (delete `DockReservations`), `world_commands.rs` (depot Enter order), `production_types.rs:248`. Prereq: A for the contact slots; the cadence can reuse `schedule_enter_retry`.

**Group D — receiver class effects (07.37)**
5. Two-sided `+0x418` (`dock_entered_with` on both entities) + BREAK→0x19 cascade + Techno echo of 0x18/0x19, Unit BREAK `Return ⇒ Guard`. Size S–M. Files: `radio/receive.rs`, `game_entity.rs`, `war_factory_exit.rs`, `production_spawn.rs:308`.

**Group E — lifecycle exactness (07.37, dormant)**
6. `Destroy(false)` non-ally-only BREAK at post-mortem (needs `is_ally` in sim — combat already has one: `grep is_ally src/sim/combat`); PointerExpired slot null in `notify_pointer_expired`. Size S. Files: `lifecycle.rs:2083`, `:2568`.
7. Sender-side HELLO eviction BREAK (`transmit_hello` → `transmit(Break, evicted)` before dispatch). Size XS. File: `radio/mod.rs:112..`.

**Group G — documentation/label hygiene (no code)**
8. Relabel `0x0065AE30` from `PathType__Has_Valid_Steps` to `RadioClass::In_Radio_Contact` (Ghidra write needs an authorized single-writer task) and sweep the ~30 docs + Rust comments (`grep -rn Has_Valid_Steps docs src`). The wrong label currently makes `Mission_Deploy_Building` state-4 BREAK look path-conditional; it is contact-conditional.
9. Amend RADIO_0X12 report §3/§8: `+0xC8..+0xD0` writes are overwritten by `Mission_Dispatch`'s epilogue on the refinery path.

## 4. Uninspected / unknown

- `MissionClass::Commence` timer semantics (first CAN_DOCK offset after HELLO, M6).
- `FootClass::Find_Nearby_Passable_Cell` occupancy flags (M7); `FootClass::Set_Destination_Internal` path behavior into the opened footprint cell (M8).
- `UnitClass::Mission_Deploy_Building @ 0x0073D630` beyond the state-4 asm window 0x0073E1F0..E28F; `ReleaseDockedHarvester`; `UndockUnit` (interrupt paths).
- Which `Destroy(bool)` variant the exact-zero kill path invokes; `BuildingClass::ChangeOwner` contact retention.
- Senders/receivers of 0x0B/0x0C/0x0D/0x11/0x1A/0x1B/0x1D/0x1E/0x1F/0x21/0x23/0x24 (aircraft, transport, bunker lanes).
- `GrandOpening` `+0x538` consumer; RadioHistory consumers (prior scan only).
- Repair depot re-target source (`+0x218`) after a NEGATORY 0x0E; helipad `0x0F` revalidation cadence vs Rust per-tick re-probe.
- `Is_Ally_ByObject` vs owner-equality for allied-house docking in team games.
- Runtime reproduction: none run (read-only scan); all timing claims are static.
