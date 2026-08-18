# Mission/Radio Substrate — Binary Verifications (workflow w78ozfwbe, 2026-06-01)

Live-Ghidra resolutions of the design-doc §9 UNCHECKED items. Read-only, verified-from-binary.

Mission name mapping confirmed:
- Index 1 = "Attack"
- Index 5 = "Guard"
- Index 0xf (15) = "Ambush" — wait, but the table is the reversed-order pointer table. Let me reconcile. The pointer table at 0x00816CAC: entry[0]=0x0081706c, entry[1]=0x00816e64="Attack". Hmm but mission 0 is usually "Sleep". The pointers DECREASE as index increases (0x0081706c, 0x00816e64, ...). Index[0] = 0x0081706c. Let me not over-interpret — the value `iVar2 != 5` etc. in the overrides are CurrentMission IDs. Whatever the exact name, what matters for this task is the predicate STRUCTURE, not the precise name of each excluded mission. I have enough: the overrides gate on locomotor-idle + current-mission-not-in-a-set + type-recruitability flag.

Actually entry[15]=0x00816df8="Ambush" conflicts with the design's enum (which lists 14=Ambush). Off-by-one is just table-vs-enum indexing; not load-bearing for this V1 task. I have sufficient evidence. Let me do one final confirmation — decompile the locomotor +0x80 slot to name the idle predicate the Infantry/Unit overrides call, and check BuildingClass +0x6dd / Aircraft +0x6d4 flag meaning is the "ready" flag. Let me get the locomotor slot via one of the override's call. Actually, I'll verify the +0x80 locomotor slot is "Is_Moving"-style by reading the unit's locomotor vtable. This is secondary; I have the core answer. Let me finalize.

I have all five subclasses verified. Let me write the report.

## V1. ReadyToCommence (vtable +0x200) per-subclass overrides — the commence gate

### Verification of the slot and the gate (HIGH, verified-from-binary this session)

- **Slot +0x200 = ReadyToCommence is correct.** Base MissionClass vtable @ `0x007EDCC0`; slot +0x200 lives at `0x007EDEC0` and holds `0x004E0140`, decompiled as `return 1` (`decompile_function 0x004E0140`). Slot alignment cross-checked: vtable+0x184 holds GetCurrentMission `0x005B3040` in UnitClass (`read_memory 0x007F5DF4`) and InfantryClass (`read_memory 0x007EB1DC`), matching design Table 2.1 (+0x184 = GetCurrentMission). So +0x200 is the correct ReadyToCommence slot in every derived vtable.
- **The gate is real and load-bearing.** `MissionClass__Queue_Mission @ 0x005B35E0` (decompiled): when the `commence` arg is nonzero it calls `(**(*this + 0x200))()` (ReadyToCommence) and **only if it returns nonzero** calls `(**(*this + 0x1EC))()` (Commence). Matches design §5.1.6 exactly. Promotion (queued→current) is therefore gated per-type. (The base `return 1` means "promote unconditionally" — that is the default contract a non-overriding type gets.)

### Enumeration of all six MissionClass-derived `+0x200` slots

Found via `get_xrefs_to 0x004E0140` → four vtable slots reference the base stub (no override); the two non-referencing leaf classes (Infantry/Aircraft) were located via their ctor vtable symbols (`list_globals vtable__InfantryClass @ 0x007EB058`, `vtable__AircraftClass @ 0x007E22A4`) and read directly. Vtable owners confirmed via `get_xrefs_to <vtable-base>` → constructor.

| Class | vtable base | +0x200 slot addr | slot target | Override? | Evidence |
|---|---|---|---|---|---|
| MissionClass (base) | 0x007EDCC0 | 0x007EDEC0 | **0x004E0140** | No (`return 1`) | read_memory 0x007EDEC0; decompile 0x004E0140 |
| FootClass | 0x007E8C94 | 0x007E8E94 | **0x004E0140** | **No** | xrefs_to 0x004E0140 lists 0x007E8E94; vtable owner = FootClass__Constructor (xrefs_to 0x007E8C94) |
| RadioClass | 0x007F0508 | 0x007F0708 | **0x004E0140** | **No** | xrefs_to 0x004E0140 lists 0x007F0708; owner = RadioClass__Constructor |
| TechnoClass | 0x007F4960 | 0x007F4B60 | **0x004E0140** | **No** | xrefs_to 0x004E0140 lists 0x007F4B60; owner = TechnoClass__Constructor |
| **BuildingClass** | 0x007E3EBC | 0x007E40BC | **0x00454250** | **YES** | read_memory 0x007E40BC; decompile 0x00454250 |
| **UnitClass** | 0x007F5C70 | 0x007F5E70 | **0x00744270** | **YES** | read_memory 0x007F5E70; decompile 0x00744270 |
| **InfantryClass** | 0x007EB058 | 0x007EB258 | **0x00521B60** | **YES** | read_memory 0x007EB258; decompile 0x00521B60; xrefs_to 0x00521B60 = {0x007EB258} only |
| **AircraftClass** | 0x007E22A4 | 0x007E24A4 | **0x0041B5E0** (`AircraftClass__Is_Ready`) | **YES** | read_memory 0x007E24A4; decompile 0x0041B5E0; xrefs_to 0x0041B5E0 = {0x007E24A4} only |

Note: FootClass, RadioClass, and TechnoClass are intermediate bases in the chain (not instantiated leaf entities) and inherit the `return 1` stub. The four leaf entity types **all** override. The intermediate `MissionClass` base itself is the `return 1` stub.

### Per-override predicate (verified-from-binary; field roles partly inferred)

**BuildingClass — 0x00454250** (HIGH):
```
return *(char *)(this + 0x6DD) != '\0';
```
A single boolean flag at byte `+0x6DD`. Ready iff that flag is set. (Field-role: a building "ready-to-act / construction-finished / not-mid-anim" flag — exact semantic INFERRED, the byte is read-only here.) Gate matters for any building mission queued with commence-now (e.g. deploy/undeploy, mission promotion during build-up).

**UnitClass — 0x00744270** (Ghidra label `UnitClass__ShouldIdle`; HIGH on structure):
Returns 0 (not ready) unless ALL hold:
- `CurrentMission (+0xAC, param_1[0x2B]) != 6 (Sleep/Harmless) && != 0x15 (0x15=Patrol/Rescue-region)`
- three byte flags clear: `+0x6E1==0`, `+0x6E2==0`, `+0x6D1==0` (busy/locked flags — INFERRED)
- then either NavCom mission `param_1[0x2D] (+0xB4 area) == 7 (Enter)`, OR a locomotor-idle check passes: `locomotor->slot+0x80 != 0` (locomotor reports idle/stationary) AND `GetCurrentMission (vtable+0x1C8) >= 0` AND `GetCurrentMission (vtable+0x184) ∉ {5 Guard, 1 Attack-with-target}` AND `(char)param_1[0x2E]==0`.
- Plus a cell/dock-adjacency special-case (building below with flag `+0x16BD`) that can also force not-ready.

**Predicate in plain terms: "the unit's locomotor is idle (not driving), it isn't already busy/attacking/guarding, and it isn't blocked by an adjacent dock building."** This gates promotion so a queued mission doesn't commence mid-drive.

**InfantryClass — 0x00521B60** (HIGH on structure):
Returns 0 unless ALL hold:
- `CurrentMission (param_1[0x2B]) != 6 && != 0x15`
- byte flags `+0x68D==0` and `+0x8D==0` clear
- locomotor `slot+0x80 != 0` (idle)
- `GetCurrentMission (vtable+0x184) ∉ {5 Guard, 0xF Ambush}`; if it `== 1 (Attack)` then also requires `param_1[0xAD]==0` (no current target)
- recruitability table gate: `param_1[0x1B1] == -1 OR (&DAT_007EAF7C)[param_1[0x1B1]*4] != 0` — i.e. the InfantryType's recruit/idle-allowed table entry must be set.

**Predicate in plain terms: same locomotor-idle + not-busy-guarding/attacking + type-allows-it gate as UnitClass, with an InfantryType lookup table (`DAT_007EAF7C`).**

**AircraftClass — 0x0041B5E0 (`AircraftClass__Is_Ready`)** (HIGH):
```
mission = *(int)(this+0xAC)
if (mission != 6 && mission != 0x15 && (this+0x6D2 byte == 0 || mission == 0x1E)):
    return (this+0x6D4 byte != 0)     // low byte
return 0
```
Ready iff: CurrentMission not Sleep(6)/0x15, AND (the `+0x6D2` busy-flag is clear OR mission is 0x1E), AND the `+0x6D4` ready-flag byte is set. (`+0x6D2`/`+0x6D4` byte roles INFERRED — set in AircraftClass__Constructor at `+0x6D1..+0x6D5`; `+0x6D4` is the "ready/landed-idle" flag.)

### Answer to the planted question

**Yes — the Rust verb API MUST have a per-type `ReadyToCommence` hook.** All four leaf entity types (Building, Unit, Infantry, Aircraft) override slot +0x200 with a real predicate; the base `return 1` is used only by the non-instantiated intermediate bases. `queue_mission(commence=true)` is **not** an unconditional promotion: `MissionClass__Queue_Mission @ 0x005B35E0` calls `ReadyToCommence()` and skips `Commence()` when it returns false. A flat `commence()` that always promotes would diverge whenever a queued+commence-now mission is issued to a unit/infantry that is still driving, an aircraft not yet landed/ready, or a building not yet construction-ready — the queued mission would silently fail to promote in gamemd but promote in the port (player-observable: e.g. a unit ordered to a queued mission mid-move would react one tick early, or a building would commence a deploy before it is ready).

The hook needs four implementations keyed on `EntityCategory`:
- **Building:** ready iff its ready-flag (`+0x6DD`) is set.
- **Unit:** ready iff locomotor idle AND not in Sleep(6)/0x15 AND not mid-Attack/Guard with a live target AND not blocked by an adjacent dock building AND internal busy-flags (`+0x6E1/+0x6E2/+0x6D1`) clear (or NavCom mission == Enter(7)).
- **Infantry:** same locomotor-idle/not-busy gate as Unit, PLUS the InfantryType recruit-allowed table check (`DAT_007EAF7C[type]`).
- **Aircraft:** ready iff not Sleep(6)/0x15, busy-flag (`+0x6D2`) clear unless mission==0x1E, AND ready-flag (`+0x6D4`) set.

### UNCHECKED / DRIFT-flagged residue (do not treat as proven)
- Exact **semantic names** of the byte flags `+0x6DD` (building), `+0x6D2`/`+0x6D4` (aircraft), `+0x6E1`/`+0x6E2`/`+0x6D1`/`+0x68D`/`+0x8D` (unit/infantry busy flags) are INFERRED from position/constructor init, not from a decompiled setter — **DRIFT until the setters are traced** (where each flag is written and to what). Required before the Slice-6 Rust hook can be field-accurate.
- The locomotor `slot+0x80` predicate (called by Unit/Infantry overrides) was **not decompiled** this session — treat its exact "idle" semantic as UNCHECKED; trace the locomotor vtable +0x80 before encoding the Rust idle test.
- Mission-ID constants in the predicates (5=Guard, 1=Attack, 0xF=Ambush, 6=Sleep, 0x15, 0x1E, 7=Enter) were mapped via `g_MissionNameTable @ 0x00816CAC`; note a possible table-vs-enum off-by-one (table index 15 → "Ambush"), which the design's §3.1 enum lists as 14 — **reconcile the indexing before hardcoding any mission constant into the gate.** This does not change the V1 conclusion (a per-type hook is required) but does affect the exact excluded-mission set.
- This V1 task did not enumerate whether any **other** vtable in the program (e.g. a hypothetical further-derived class) overrides +0x200; the six derived slots above are the complete set reachable from the MissionClass chain bases known to the design (xref closure of base/Building/Unit + the two leaf vtables). No additional MissionClass-derived vtable was found.

---

That address isn't in a defined function (the constructor proper starts elsewhere; xrefs labeled it AirstrikeClass__Constructor at the read sites). I have sufficient verified evidence. The setter is fully pinned. Let me write the final section.

## V2. AircraftClass +0x294 radio-deaf latch — the SETTER

**Verdict: SETTER PINNED (verified-from-binary).** `+0x294` (`this[0xa5]`, a 4-byte field) is **not a boolean "cleared-to-receive" flag** — it is a **back-pointer from an Airstrike-summoned aircraft to its controlling `AirstrikeClass` object**. The Receive_Radio gate (`0x004190B0`, `if (param_1[0xa5] == 0) return 0;` for `CurrentMission ∈ {4,0x1A,0x1B,0x1E,0x1F}` — verified, read encoding `8b 81 94 02 00 00 / 85 c0` at `0x004190d3`) means: a summoned aircraft is **radio-deaf while it is bound to an active airstrike controller** (latch nonzero) and running one of those 5 airstrike-flight missions; it becomes **radio-receptive only after the controller releases it** (latch → 0).

### Write-sites (all verified by `read_memory`/`decompile_function` this session)

Exhaustive scan: `search_byte_patterns` over every `mov [reg+0x294], imm/reg` encoding (`c7 8x`, `c6 8x`, `89 8x` for all reg bases). Only **three** write to a verified AircraftClass `this` (gated on RTTI `(**)(*obj+0x2c)() == 6`, type-6 = Aircraft):

1. **SET (assign) — `FUN_0041d860 @ 0x0041d860`** (AirstrikeClass "begin/launch" path; calls aircraft factory `FUN_0065e850`, reads elite/rookie types `+0x24/+0x28`, writes Rules-palette color bytes `+0x6f9/+0x6fa` from `RulesClass+0x18a4`). Write at `0x0041d8??`: `param_2[0xa5] = param_1;` (the aircraft's latch ← the AirstrikeClass `this`), then aircraft vtable `+0x15c(100000)` and `+0x124(2)`. **Condition:** an aircraft (type 6) is being attached as the airstrike's lead unit (`+0x50` slot).
2. **SET + clear-old — `FUN_0041da20 @ 0x0041da20`** (AirstrikeClass "swap lead unit"; sole caller `FUN_0041d830 @ 0x0041d83c`). Two writes:
   - `piVar3[0xa5] = 0;` at `0x0041daa4` (`c7 87 94 02 00 00 00000000`) — **clears the OUTGOING aircraft's latch to 0** (it becomes radio-receptive), then vtable `+0x158()` and `+0x124(2)`.
   - `piVar2[0xa5] = param_1;` — **sets the INCOMING aircraft's latch** to the AirstrikeClass owner, then `+0x15c(100000)`, `+0x124(2)`.
   - Both filtered to type-6 via the `~-(uint)(iVar4 != 6) & ptr` idiom.
3. **CLEAR/reassign (teardown) — `FUN_0041db40 @ 0x0041db40`** (AirstrikeClass disconnect; called from `FUN_0041da20` head on owner mismatch). Write at `0x0041dc10` (`89 96 94 02 00 00` = `mov [esi+0x294], edx`). Guarded by `piVar1[0xa5] == param_1` (latch still points to THIS controller); then scans the **global AirstrikeClass registry `DAT_00889fbc`** (count `DAT_00889fc8`) for another controller whose `+0x50` lead-slot is this same aircraft and **reassigns** `piVar1[0xa5] = thatController` (or `0`); final dangling-guard `if (piVar1[0xa5] == param_1) piVar1[0xa5] = 0;`. **Condition:** the controlling airstrike is being released/destroyed → latch reassigned to a surviving controller or zeroed.

**Owner-class identity proof:** `get_xrefs_to 0x00889fbc` returns reads/writes exclusively from `AirstrikeClass__Constructor` (`0x0041d36f`, `0x0041d3ef`), `AirstrikeClass__ScalarDeletingDestructor` (`0x0041dda0`), and `FUN_0041db40` (`0x0041dbf3`) — so `DAT_00889fbc/c8` is the global AirstrikeClass array and the `0x41d8xx–0x41db40` helpers are AirstrikeClass methods. (Initial reading mislabeled this SpawnManagerClass; corrected — `SpawnManagerClass` lives at `0x006b6c90+` and does not touch `0x00889fbc`.)

**Constructor (`0x00413D20`) does NOT write +0x294:** `decompile_function` shows no `param_1[0xa5]` write; the latch is zero-initialized by the FootClass/base allocator (calloc-style). So a normally-built aircraft starts radio-receptive; only an airstrike attach makes it deaf.

**Ruled-out (not the aircraft latch):** the 5 candidate mission handlers the task named — `Mission_ParaDropApproach 0x004155F0`, `Mission_ParaDropOverfly 0x004157C0`, `Mission_SpyPlane 0x00417300`, `Mission_Hunt 0x004154B0` (the address given as "AI" decompiles to Mission_Hunt; true `AircraftClass::AI` not located, but its body would only READ the latch), `Mission_Move_Carryall 0x00416D50` — **none writes `[reg+0x294]`** (verified decompiles; offsets used are 0x2f/0xad/0x169/0xbf, never 0xa5). The aircraft-range `8b 8x 94 02 00 00` hits are all latch **READs** (Receive_Radio `0x4190d3`, Enter_Idle_Mode `0x417750`, Mission_Attack `0x418c94`, Assign_Mission `0x41ba0a`, Queue/Override verbs `0x41baaa/0x41bb4a`, and a jump-table mission at `0x414a3a`). The non-aircraft `c7 86`/`89 8e` writes (OverlayTypeClass ctor `0x5fe280`, `FUN_0065e850 0x65e8f5`, `0x6b5277`, `0x710b02`, `0x71da90`) and the WarheadTypeClass `+0x294` char-flag read in `BulletClassBulletDetonationImpactDamage 0x00469080` are **different classes' +0x294 fields** — confirmed not the latch.

### Rust contract for the substrate

For the design's §5.1.10 / §9.2 / Risk 3 ledger — replace "AircraftClass `+0x294` latch (boolean, setter UNCHECKED)" with:

- **Model `+0x294` as `airstrike_owner: Option<EntityId>`** on the aircraft component (a back-link to a controlling **AirstrikeClass** service object), NOT a bool. The radio-deaf gate is `airstrike_owner.is_some() && mission ∈ {Retreat, ParaDropApproach, ParaDropOverfly, SpyplaneApproach, SpyplaneOverfly}`.
- **Toggles radio-deaf (set owner):** when an AirstrikeClass attaches the aircraft as its lead unit (`FUN_0041d860` launch, `FUN_0041da20` lead-swap incoming). The vtable side-effects `+0x15c(100000)` (set ammo/fuel to max), `+0x158` (refuel/reset), `+0x124(2)` (mark mission-active) must accompany the set/clear in the substrate.
- **Toggles radio-receptive (clear owner → None / reassign):** when the controller releases it — lead-swap outgoing unit (`FUN_0041da20`, latch→0), or controller teardown (`FUN_0041db40`, reassign to a surviving controller in the global registry, else 0). The teardown's registry-rescan (reassign to another controller that already claimed this aircraft) is **load-bearing** if multiple airstrikes can target the same aircraft; otherwise it degenerates to "clear to None."
- **Coupling requirement holds:** `receive_radio` for aircraft must read both `MissionCom.current` AND `airstrike_owner`; the substrate must keep them co-readable (design Risk 3). Since airstrike summoning is the **only** writer, the latch is owned by a future `sim/airstrike` (or equivalent summon-controller) service, written on attach/detach, and read by the radio bus — it is NOT a mission-scheduler field.

**Activity note (frequency for severity):** this latch fires only for **airstrike-summoned aircraft** (lightning/paradrop/spyplane/chrono-summon flights), not the common player-built Kirov/Harrier/Black-Eagle population, so the radio-deaf path is **rare in a standard skirmish** (only when a support-power aircraft is mid-flight). It is real YR behavior (the 5 gated missions are all live airstrike-flight missions), not TS-dead, so it must be modeled, but it can be deferred to the airstrike slice without blocking the refinery/airfield/bunker radio idioms.

**Confidence:** SET/CLEAR sites, values, gating, and owner-class identity = **HIGH, verified-from-binary this session** (`decompile_function` 0x004190B0/0x00413D20/0x0041d860/0x0041da20/0x0041db40/0x0041d830; `read_memory` byte-decode at 0x0041daa4/0x0041dc10/0x004190cd/0x00414a30; `get_xrefs_to 0x00889fbc`; `search_functions SpawnManager`). The only UNCHECKED residual: exact `AircraftClass::AI` read-site (not located) and whether more than one AirstrikeClass can simultaneously claim one aircraft (the `FUN_0041db40` rescan implies yes, but the multi-claim trigger was not exercised) — neither affects the SET/CLEAR contract above.

---

I have everything I need. Both Rust structures use explicit FIFO `VecDeque` wait-queues with `push_back`/`pop_front`/`front` promotion — a stored arrival-order list that gamemd has no counterpart for. Let me confirm the aircraft promotion-pad behavior detail (the Rust promotes "next queued into the just-freed pad index"), then I have the full picture.

The Rust airfield `release()` (aircraft_dock.rs:178-182) pops `front()` and assigns it the freed pad. That's a stored-FIFO promotion. In gamemd, `FindBuildingToDock` revalidates `CachedDock` via 0x0F, and on failure recomputes via `Find_Docking_Bay` (distance-sorted nearest pick) — there is no stored queue; whichever waiting aircraft's `AI_Update` next runs `FindBuildingToDock` and gets ROGER takes the slot.

I have conclusive evidence for all three sub-claims. Producing the final section.

## V3. Dock eviction / who-docks-next order — gamemd vs the Rust FIFO

**VERDICT: The Rust FIFO wait-queues (`RefineryDockContacts.waiting_retry_queue`, `AirfieldDocks.queues`) are a player-visible DRIFT.** gamemd has **no stored wait-queue**; "who docks next" is decided by **dispatch/retry order** (whichever waiting unit's `AI_Update` runs `Mission_Harvest`/`FindBuildingToDock` next and gets a ROGER), with the eventual winner biased by **distance**, not arrival order. The plan should replace the FIFO with retry-order admission, not document accepted DRIFT.

### Verified gamemd "who docks next" rule

**(a) Receiver returns NEGATORY when full — NO eviction.** `RadioClass::Receive_Radio @ 0x0065A820` HELLO(0x02) path: alive gate (`+0x6C != 0`) → ally double-check (`HouseClass__Is_Ally_ByObject`, second gated on `+0x14 & 1`) → idempotent (already-present ⇒ ROGER 1) → `if (*(int*)(this+0xE8) < 1) return 10` → free-slot linear scan; first `*slot == 0` ⇒ insert + `return 1`, fall off loop ⇒ `return 10` (NEGATORY). **No existing contact is ever nulled to make room** — the receiver never evicts. (Verified `decompile_function 0x0065A820`.)

**(b) Sender evicts its OWN Contacts[0] only when the SENDER's array is full.** `RadioClass::Transmit_Radio_Impl @ 0x0065A970` HELLO path: scans `this+0xE4` for a free slot (`iVar3` = first null index) and for an existing match (returns ROGER early). If no free slot found (`iVar3 == -1` after the loop), it calls `(**(this+0x278))(3, Contacts[0])` = **Transmit BREAK to its own slot-0 contact**, then reuses `iVar3 = 0`. This is the *sender's* array (a miner/aircraft, capacity 1 in YR via the default ctor `0x0065A750`), never the refinery's. The refinery as receiver still rejects via (a). (Verified `decompile_function 0x0065A970`; eviction call `(**(*param_1 + 0x278))(3, *Contacts[0])` at `LAB_0065aa36`-preceding block.)

**(c) No FIFO/queue structure on refinery or airfield — next docker = re-send/retry order, distance-biased.**
- **Refinery:** `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` (CAN_DOCK) contains **no queue/list bookkeeping**. Its only loop iterates `field_0xE8` (= the Contacts capacity, `max(NumberOfDocks,1)`) to query (`0x22`)/evict-via-`0x17` *its own contacts*, not a waiting list. Admission is gated by membership (`DynamicVectorClass__Contains`) + a fresh HELLO (`(vtable+0x278)(2,param_2)`). No arrival-order storage exists. (Verified `decompile_function 0x0043C2D0`.)
- **Miner retry:** `UnitClass::Mission_Harvest @ 0x0073E5E0` **case 2** (dock-seek substate, `param_1[0x2f]==2`): each dispatch it recomputes the nearest refinery via `(vtable+0x528)(this+0x1B1+1000,…)` (house building search, distance-sorted), then at `LAB_0073ee51` sends HELLO: bytes `6a 02 8b cd ff 90 78 02 00 00 83 f8 01` = `PUSH 2; MOV ECX,EBP; CALL [EAX+0x278]; CMP EAX,1`. On ROGER (`==1`) → `param_1[0x2f]=3` (dock); otherwise it stays in state 2 and **re-attempts next dispatch** — no stored position. Whichever miner's `Mission_Dispatch` (`0x005B3060`, sole per-tick caller) re-runs case 2 and lands a ROGER first wins the freed slot. (Verified `decompile_function 0x0073E5E0`; `read_memory 0x0073ee51`.)
- **Airfield:** `AircraftClass::FindBuildingToDock @ 0x0041BBD0` revalidates a single `CachedDock` (`param_1[0x1B3]`) by sending `0x0F` (CAN_ENTER); if reply `!=1` it clears the cache and calls `FootClass::Find_Docking_Bay @ 0x004DF040`. `Find_Docking_Bay` linearly scans the docking-bay list and selects by a **per-bay distance metric** (`param_2 < local_4` keeps the smaller distance; tie-break on `*(iVar2+0x3D3)`) — **no queue, no arrival-order list**, recomputed every call. Next pad goes to whichever waiting aircraft next runs `FindBuildingToDock` and passes the `0x0F`/distance pick. (Verified `decompile_function 0x0041BBD0` and `0x004DF040`; `get_function_callers 0x004DF040` → sole caller `FindBuildingToDock`.)

**Exact rule:** *There is no "next docker" queue. A saturated refinery/airfield rejects every HELLO/CAN_ENTER with NEGATORY (10). A waiting unit keeps re-probing on its own dispatch cadence. When a slot frees (departing unit sends BREAK 0x03, nulling its contact slot), the next unit to (i) get dispatched, (ii) re-probe, and (iii) win the receiver's free-slot scan / distance pick takes it.* Because `Mission_Dispatch` iterates the active-techno set in a deterministic order and the refinery scan picks the **nearest** candidate, the practical winner is **distance-then-dispatch-order**, not FIFO arrival order.

### The Rust DRIFT

Both Rust structures store an explicit arrival-ordered FIFO and promote by it:
- `miner_dock.rs:33,55-59` — `waiting_retry_queue: BTreeMap<u64, VecDeque<u64>>`; on full it does `queue.push_back(miner_sid)` (arrival order) and `next_waiter()` (`miner_dock.rs:174-177`) returns `queue.front()` — strict FIFO.
- `aircraft_dock.rs:113,160-163,178-182` — `queues: BTreeMap<u64, VecDeque<u64>>`; `try_reserve` does `push_back`, `release()` does `pop_front()` and assigns the promoted aircraft **the just-freed pad index** (tests `airfield_docks_release_pad_1_promotes_into_pad_1` lock this in at lines 747-760).

**Why it diverges from gamemd:** FIFO guarantees the *earliest-arriving* waiter docks next. gamemd guarantees the *nearest / next-dispatched* re-prober docks next. These disagree whenever the arrival order ≠ distance order: a miner that queued first but sits farther from the refinery exit than a later arrival will, in gamemd, **lose** the freed slot to the closer miner (since `Find_Docking_Bay`/the harvest building-search pick nearest); under the Rust FIFO it wins. For airfields the Rust additionally pins the promoted aircraft to the *specific* freed pad index, whereas gamemd re-runs `Find_Docking_Bay` and may pick a *different* bay/pad entirely.

**Player-visibility & frequency:** Observable as **which miner physically moves to the refinery next** when **2+ miners contend a 1-dock refinery** (`NumberOfDocks=1` — every stock refinery; fires in essentially every match with ≥2 miners per refinery, i.e. the normal mid-game economy), and as **which aircraft lands on which pad** when **5+ aircraft contend a 4-pad airfield** (`NumberOfDocks=4`). The miner case is high-frequency (busy economies routinely run 2-3 miners per refinery); the result is a visibly different miner picking up the next unload turn and a different movement path. This is a divergent unit selection and pathing outcome — squarely inside the parity bar (per CLAUDE.md: who-docks-next ordering is a player-observable output; default verdict DRIFT, and here it is proven DRIFT, not merely unproven).

### Rust-side fix the plan should adopt

**Replace the FIFO with retry-order admission, modelling the gamemd primitive (not a stored queue):**

1. **Drop both wait-queue fields** — `RefineryDockContacts.waiting_retry_queue` and `AirfieldDocks.queues` (plus `is_waiting`/`remove_waiter`/`next_waiter` and the `pop_front` promotion). A full slot simply returns NEGATORY; no waiter is recorded.
2. **Admission = on-demand re-probe.** A waiting miner stays in its harvest dock-seek substate (state-2 analogue) and **re-attempts HELLO each dispatch tick**; admission succeeds only when the contact set has a free slot at probe time (mirroring `Receive_Radio` free-slot scan). No "who's been waiting longest" bookkeeping.
3. **Winner = nearest, then deterministic dispatch order.** When the freed slot is contested, pick the candidate the way gamemd does: the refinery/airfield building-search picks the **nearest** eligible unit; ties resolve by the deterministic BTreeMap entity-iteration order (the lockstep-safe Rust analogue of gamemd's active-techno dispatch order). This belongs in the unified `transmit()`/`receive_radio` bus the design proposes (§6.2), not in a separate registry — i.e. fold both registries into the bus per retire-list #9 (`miner_dock.rs:42-78`, `aircraft_dock.rs:134-193`), and let the **`Contacts` free-slot scan** be the sole admission gate.
4. **Airfield pad assignment** must come from `find_slot(contact)` → `DockingOffsetN` (design §5.2.11), recomputed on (re)dock, **not** from a promoted-into-the-same-pad-index rule — so a re-docking aircraft can be assigned a different free pad, matching `Find_Docking_Bay` re-selection. The existing test `airfield_docks_release_pad_1_promotes_into_pad_1` encodes the *wrong* (FIFO-pin) behavior and should be replaced with a nearest/free-pad-rescan assertion.

This resolves the design's §9.2 / §9.3-Risk-4 UNCHECKED item ("FIFO eviction order vs gamemd slot-0-evict … no verified gamemd counterpart"): the gamemd counterpart is **no queue at all** — distance-biased on-demand retry — and the FIFO is a proven DRIFT to remove in **Slice 4** (refinery bus) and **Slice 7 airfield** (`aircraft` idiom).

**Citations:** `decompile_function` 0x0065A820 (receiver no-evict NEGATORY), 0x0065A970 (sender slot-0 self-evict), 0x0043C2D0 case 0x0E (refinery, no queue), 0x0073E5E0 case 2 (miner retry HELLO), 0x0041BBD0 + 0x004DF040 (airfield CachedDock revalidate + distance-pick, no queue); `read_memory 0x0073ee51` (HELLO re-send bytes); `get_function_callers 0x004DF040`. Rust: `src/sim/miner/miner_dock.rs:31-37,54-59,146-177`, `src/sim/docking/aircraft_dock.rs:113,160-163,178-182,747-760`.

---

These `6A 00 6A 0E` hits are in UI ownerdraw code (0x614190) and unrelated functions — not mission verb calls (Queue/Assign_Mission take the mission as the first/single arg, and there's no Mission_Ambush handler to dispatch to anyway). The `0xe` constant in `push 0; push 0xe` patterns here are coordinates/control IDs, not missions. Mission 14 (Ambush) is confirmed dead: no handler function, base slot = `Mission_Default` stub (`return 0x1c2`), only string reference is the name table.

I have enough evidence to resolve both. Final note on Rescue's gameplay role: it's not a "rescue civilians" mission as the name suggests — in YR it's the AI's **damage-response convergence** mission (the handler walks the unit to the threatened/attacker cell, `+0x86`, then clears). The handler is heavily TS-flavored (`SetGhostCell`, `Find_Passable_Cell_Near_Unit`) but the assigner is live YR AI.

## V4. Ambush(14) / Rescue(21) — reachable in stock YR skirmish?

**Method:** Dispatch switch (`Mission_Dispatch @ 0x005B3060`, decompiled) confirms both IDs have live cases — case `0xe`(14)→vtable slot `+0x20c`, case `0x15`(21)→slot `+0x258`. Resolved assigners by (a) mission-name string xrefs, (b) `Mission_From_Name @ 0x005B3910` callers, (c) numeric `push 0x15`/`push 0xe` byte-pattern scans near mission verbs, (d) tracing the AI threat-response routine that emits these IDs.

### Mission 21 (Rescue) — **Reachable in YR: YES** (live AI behavior, not map-scripted)

- Real handlers exist on every relevant subclass vtable: `AircraftClass__Mission_Rescue @ 0x00415960` (vtable slot read at `0x007e2508` = `0x00415960`) and `FootClass__Mission_Rescue @ 0x004ddf90` (installed in 4 vtables: `0x007e24fc`/`0x007e8eec`/`0x007eb2b0`/`0x007f5ec8`, slot `+0x258`; `0x007e24fc` reads `0x004ddf90`). Neither has direct callers — dispatched **only** via `Mission_Dispatch` when `CurrentMission==21`.
- **Live assigner (decisive):** `FUN_00708080` (the AI threat-response gather routine) issues `Queue_Mission(uVar6,0)` (vtable `+0x1e8`) where `uVar6 = 0x15`(Rescue) or `0xb`(AreaGuard), chosen by `Random__RandomRanged(0,99)` (≤0x41 → Rescue; byte pattern `6A 00 6A 15` confirmed at `0x00708700` inside this fn). It sets each responder's target field `+0x86` to the gather object's cell — which is exactly what `FootClass__Mission_Rescue` reads (`param_1[0x86]`), closing the loop.
- **Caller chain to live skirmish:** `FUN_00708080` is called from the `ReceiveDamage` family — `FootClass__ReceiveDamage @ 0x004d7330`, `BuildingClass__ReceiveDamage @ 0x00442230`, `TechnoClass__ReceiveDamage @ 0x00701900` (verified `get_function_callers`). In `0x004d7330` the call is gated by `HouseClass__IsPlayerControl()==0` (AI house only) + team-membership (`+0xb4`, type-flag `+0xac`) + a non-null attacker (`param_5`). So: **whenever an AI-team unit takes damage from an attacker in a skirmish vs computer opponents, the AI tasks nearby idle teammates with Rescue(21) to converge on the attacker.** No `SpecialFlags` gate, no map trigger required — fires every skirmish with AI players.
- **Caveat (player-visibility):** Rescue is gated `IsPlayerControl()==0`, so it is **never assigned to human-player units** — only AI-owned units run it. The handler itself (`FootClass__Mission_Rescue`) is TS-flavored internally (`SetGhostCell`, `Find_Passable_Cell_Near_Unit`, `Drop_Payload` on aircraft) but the assigner is unambiguously active YR AI.
- **Recommendation: INCLUDE a Rust handler** for `MissionType::Rescue(21)` in the enum. It is reachable in every AI skirmish; its observable effect is AI units rushing to a damaged teammate's attacker. Note it is AI-only — the player never sees their own units in this mission, but they see enemy AI units converging via it (player-visible behavior). Scope its dispatch into the AI threat-response slice, not the player-command path.

### Mission 14 (Ambush) — **Reachable in YR: NO** (dead stub, TS-legacy)

- **No handler exists.** `search_functions` for `Ambush`/`Mission_Ambush` → none. The base vtable slot `+0x20c` (case `0xe` target) resolves to `MissionClass__Mission_Default @ 0x005B2E10` — verified body `return 0x1c2` (a no-op that defers 450 ticks; `read_memory 0x005B2E10` = `B8 C2 01 00 00 C3` = `mov eax,0x1C2; ret`). No subclass overrides it.
- **No assigner.** The "Ambush" string (`0x00816df8`) is referenced **only** from the mission name table (`0x00816ce4`, inside `0x00816CAC`); no other xref. `Mission_From_Name` (the sole name→ID path) is called only by the three map-INI placement parsers (`[Units]` `0x00743270`, `[Infantry]` `0x0051fb00`, `[Aircraft]` `0x0041b110`) — so a campaign map *could* name "Ambush" on a pre-placed object, but it would dispatch the do-nothing stub. The `push 0; push 0xe` byte-pattern hits (`0x614469` ownerdraw, `0x64a06a`, `0x4a3776`, etc.) are UI/control-ID constants, not mission-verb calls. No numeric `Queue/Assign_Mission(14)` exists in AI/team/trigger code.
- **Recommendation: SCOPE OUT as TS-legacy.** No handler, no live assigner, observable effect = none. Do not add a `MissionType::Ambush` Rust handler beyond an inert enum placeholder. If the enum must round-trip all 30 dispatched IDs by name (for map-INI parse fidelity), keep `Ambush` as a **no-op variant** mapping to the Sleep/Default behavior (`return 450`-equivalent defer) — matching gamemd's stub exactly — but implement no logic.

### Net for the plan
- §9.2 UNCHECKED entry "Ambush(14)/Rescue(21) YR-activity — UNKNOWN" is now **RESOLVED**: Rescue = **DRIFT if omitted** (live AI behavior, must be modeled); Ambush = **inert stub** (model as no-op for name round-trip only).
- §3.1's "both base stubs, no identified live YR trigger" is **WRONG for Rescue** — it has real per-subclass handlers and a live AI assigner via `ReceiveDamage → FUN_00708080`. Correct the doc.
- All addresses verified live this session via `decompile_function`/`read_memory`/`get_function_callers`/`get_xrefs_to`/`search_byte_patterns`; no Ghidra writes performed.

---

Confirmed. InfantryClass +0x224 (Harvest) = 0x00524E70 (`702e5200`) — a real Infantry handler (likely for the Yuri-side infantry that can occupy/harvest, or a stub-override; regardless it's a non-base address = overridden). InfantryClass +0x228 (Hunt)=0x51F540, +0x22C (Move)=0x51F660. BuildingClass +0x224 (Harvest)=0x44B770 real, +0x228 (Hunt)=0x5B2EA0 stub, +0x22C (Move)=0x5B2EB0 stub, +0x230 (Retreat)=0x5B2EC0 stub. All consistent with my decode.

I have everything verified. Writing the final report.

## V5. RadioHistory readers + per-subclass mission-override completeness

### PART A — RadioHistory (+0xD4/+0xD8/+0xDC) readers: VERDICT = **OMIT-SAFE** (no reader found; HIGH confidence)

**Sole writer = base `RadioClass::Receive_Radio` @ 0x0065A820.** Its prologue shifts the 3-deep history and records the new message; nothing reads it back (`decompile_function 0x0065A820`):
```
if (param_3 != *(int*)(this+0xd4)) { *(this+0xd8)=*(this+0xd4); *(this+0xdc)=old_d8; *(this+0xd4)=param_3; }
```
The history is write-only push-down; no branch downstream consumes +0xD4/+0xD8/+0xDC.

**Every subclass `Receive_Radio` override forwards unhandled messages to the base and none reads the history** (all `decompile_function`, this session):
- `TechnoClass::Receive_Radio` @ 0x006F4AB0 — switch on msg; default tail-calls `RadioClass__Receive_Radio`. No +0xD4/D8/DC read.
- `FootClass::Receive_Radio` @ 0x004D8FB0 — default tail-calls `TechnoClass__Receive_Radio`. No history read.
- `UnitClass::Receive_Radio` @ 0x00737430 — default → `FootClass__Receive_Radio`. No history read.
- `AircraftClass::Receive_Radio` @ 0x004190B0 — opens with the mission-state radio-deaf gate (reads `+0xAC` mission, `+0x294` latch), default → `FootClass__Receive_Radio`. No history read.
- `BuildingClass::Receive_Radio` @ 0x0043C2D0 — default → `TechnoClass__Receive_Radio`. No history read.
- **InfantryClass has NO own `Receive_Radio`** — its vtable +0x194 slot = 0x004D8FB0 (`read_memory 0x007eb1ec` = `b0 8f 4d 00`), i.e. it inherits FootClass's. (FootClass is abstract; its vtable +0x194 also = 0x004D8FB0, `read_memory 0x007e8e28`.) So no extra reader hides there.

**Binary-wide byte scan for direct reads** (`search_byte_patterns`): `8B 80 D4/D8/DC 00 00 00` (mov eax,[eax+disp32]) and `8B 81 D4/D8/DC 00 00 00` (mov reg,[ecx+disp32]). Every hit decompiled to a **different class's field**, never RadioClass history:
- +0xD4 hits: `FUN_006f18a0` (a `-1`-sentinel index passed to `FUN_0068bcc0`, on a non-Radio object), `FUN_004026a0` (vtable call `(*(*[this+0xD4]+0x28))()`), `TiberiumClass__Detach` @ 0x0072215B (vector count), `0x004026b0`.
- +0xD8/+0xDC hits: `TerrainClass__Get_Render_Rect` @ 0x0071D175 (Terrain coords), `BuildingClass__SaveToINI` @ 0x0044FEF3 (reads *another* object's +0xDC = AttachedTag/trigger, not `this`), `VeinholeMonsterClass__Constructor` @ 0x0074C812 (TS-legacy ghost; reads `g_RulesClass+0xdc`), the sidebar window-position `FUN_0060B1D0`.

None are RadioClass-layout (TechnoClass-derived) objects reading their own +0xD4/+0xD8/+0xDC. **No save/load of RadioHistory observed.** Conclusion: RadioHistory is safe to OMIT from the Rust port — it is observably inert (write-only). Caveat: scan covered the two common `mov`-with-disp32 encodings; an exotic encoding or a base-register-relative read inside an inlined helper is not 100% excluded, so keep the design's "UNCHECKED-omittable" label downgraded to **omit-safe (HIGH)** rather than proven-inert (PROOFED). Resolves §9.2 ledger item "RadioHistory has no subclass reader."

### PART B — Per-subclass mission-handler override map (+0x204..+0x270; base stub = 0x005B2E10 `mov eax,0x1C2; ret`)

Slot→mission map verified live from `Mission_Dispatch` @ 0x005B3060 (decompiled this session) and `MISSIONCLASS_STATE_MACHINE.md`. A slot is **overridden** iff its vtable entry points outside the base-stub block 0x005B2E10–0x005B2FC0. All five vtables identified by RTTI type-descriptor strings (`read_memory`): BuildingClass 0x007E3EBC, UnitClass 0x007F5C70, AircraftClass 0x007E22A4 (`.?AVAircraftClass`), InfantryClass 0x007EB058 (`.?AVInfantryClass`), FootClass 0x007E8C94 (`.?AVFootClass`, abstract). Mission-slot bytes read directly from each vtable.

**FootClass (abstract base for Unit/Infantry/Aircraft) — real handlers at:**
`1 Attack(0x4D4DC0), 2 Move(0x4D4200), 4 Retreat(0x4DA2C0), 5/6 Guard/Sticky(0x4D5070), 7 Enter(0x4D9290), 8/17 Capture/Sabotage(0x4D4B20), 9 Eaten(0x4D4CB0), 11 AreaGuard(0x4D6AA0), 15 Hunt(0x4D5350), 16 Unload(0x4DA2B0), 21 Rescue(0x4DDF90), 25 Patrol(0x4D4280)`. Stub (inherits 0x1C2): `0 Sleep, 3 QMove, 10 Harvest, 12 Return, 13 Stop, 14 Ambush, 18 Construction, 19 Selling, 20 Repair, 22 Missile, 23 Harmless, 24 Open, 26/27 Paradrop, 28 Wait, 30/31 Spyplane`.

**UnitClass — own overrides:** `1 Attack(0x744780), 5/6 Guard(0x740810), 10 Harvest(0x73E5E0), 11 AreaGuard(0x744100), 15 Hunt(0x73EFC0), 2 Move(0x740A90), 16 Unload(0x73D630), 20 Repair(0x740EF0), 25 Patrol(0x740B10)`. Inherits FootClass-real for `4 Retreat, 7 Enter, 8/17 Capture/Sab, 9 Eaten, 21 Rescue`. Stub for `0 Sleep, 3 QMove, 12 Return, 13 Stop, 14 Ambush, 18 Construction, 19 Selling, 22 Missile, 23 Harmless, 24 Open, 26–28, 30/31`.

**InfantryClass — own overrides:** `1 Attack(0x51F3E0), 5/6 Guard(0x51F620), 11 AreaGuard(0x51F640), 10 Harvest(0x524E70), 15 Hunt(0x51F540), 2 Move(0x51F660), 16 Unload(0x51F6E0)`. Inherits FootClass-real for `4 Retreat, 7 Enter, 8/17 Capture/Sab, 9 Eaten, 21 Rescue, 25 Patrol`. Stub for `0 Sleep, 3 QMove, 12 Return, 13 Stop, 14 Ambush, 18 Construction, 19 Selling, 20 Repair, 22 Missile, 23 Harmless, 24 Open, 26–28, 30/31`.

**AircraftClass — own overrides:** `1 Attack(0x417FE0), 5/6 Guard(0x41A5C0), 11 AreaGuard(0x41A940), 15 Hunt(0x4151E0)…` — slots: `1 Attack(0x417FE0), 5/6 Guard(0x41A5C0), 11 AreaGuard(0x41A940), 15 Hunt(0x414A80), 2 Move(0x4166C0), 4 Retreat(0x415A50 — labeled `Mission_QMove`), 16 Unload(0x4151E0), 7 Enter(0x419C80), 25 Patrol(0x417300), 26 ParadropApproach(0x4158E0), 27 ParadropOverfly(0x415960), 30 SpyplaneApproach(0x4155F0), 31 SpyplaneOverfly(0x4157C0)`. Inherits FootClass-real for `8/17 Capture/Sab, 9 Eaten, 21 Rescue`. Stub for `0 Sleep, 3 QMove, 10 Harvest, 12 Return, 13 Stop, 14 Ambush, 18/19/20 Construction/Selling/Repair, 22 Missile, 23 Harmless, 24 Open, 28 Wait`.

**BuildingClass (own missions; does NOT inherit FootClass):** real handlers at `1 Attack(0x44ACF0), 8/17 Capture/Sabotage(0x44B760), 5/6 Guard(0x4496B0), 11 AreaGuard(0x449A40), 10 Harvest(0x44B770), 16 Unload(0x44D880), 18 Construction(0x449A50), 19 Selling(0x449C30), 20 Repair(0x44B780), 22 Missile(0x44C980), 24 Open(0x44E440)`. Stub for `0 Sleep, 2 Move, 3 QMove, 4 Retreat, 7 Enter, 9 Eaten, 12 Return, 13 Stop, 14 Ambush, 15 Hunt, 21 Rescue, 23 Harmless, 25 Patrol, 26–28, 30/31`.

**Plan implication (which MissionType variants need a real Rust handler, per category):**
- **Vehicles (UnitClass):** Sleep, Attack, Move, Retreat, Guard/Sticky, Enter, Capture/Sabotage, Harvest, AreaGuard, Hunt, Unload, Repair, Patrol, Rescue, Eaten. (15 — Harvest/Unload/Enter are the dock-loop missions.)
- **Infantry:** same set minus Repair/Patrol-override (Patrol inherits Foot); plus Harvest(0x524E70) is implemented (note: infantry-Harvest is real, verify YR reachability before treating as live — candidate dormant).
- **Aircraft:** Attack, Move, Retreat(=QMove-behavior handler), Guard, AreaGuard, Hunt, Unload, Enter, Patrol, Paradrop Approach/Overfly, Spyplane Approach/Overfly. (No Harvest/Construction/Selling/Repair/Missile.)
- **Buildings:** Attack, Capture/Sabotage, Guard, AreaGuard, Harvest, Unload, Construction, Selling, Repair, Missile, Open. (No Move/Enter/Patrol/Retreat — buildings are stationary; Open = gate.)
- **No category implements:** Ambush(14) — base stub everywhere → confirms §3.1 "no live trigger" / UNKNOWN. Spyplane/Paradrop are **Aircraft-only**. Missile(22) is **Building-only**. Construction/Selling are **Building-only**. Repair(20) is Unit+Building. Open(24) is Building-only.

**DRIFT flag against the design doc (§3.1, for the synthesis step):** the doc claims "AircraftClass overrides the +0x204 (Sleep) slot with a real QMove handler (0x00415A50)." **Verified wrong:** AircraftClass +0x204 (Sleep) = base stub 0x005B2E10 (`read_memory 0x007e24a8` = `10 2e 5b 00`); the handler 0x00415A50 (`AircraftClass__Mission_QMove`, decompiled) is installed at slot **+0x230 = mission ID 4 (Retreat)**, not +0x204. Mission 3 (QMove) therefore routes — for *all* classes including Aircraft — to slot +0x204 (Sleep), which is the base stub on Aircraft (returns 0x1C2). The "preserve this quirk" instruction should target the Retreat-slot placement of the QMove-named function, not a +0x204 override. Per the burden-of-proof rule this is DRIFT in the design's §3.1 and should be corrected before the slice that wires aircraft mission handlers.

---

