# Building Radio 0x15 Unload Side Effects - Reswarm Research Report

**Address(es):** `BuildingClass::Receive_Radio @ 0x0043C2D0`, stock `0x15` block `0x0043C788..0x0043C7B2`; `MissionClass::Queue_Mission @ 0x005B35E0`; `UnitClass::Mission_Deploy_Building @ 0x0073D630`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Stock YR `HARV/CMIN -> GAREFN/NAREFN` radio `0x15` unload handoff: immediate building receiver side effects, mission/status transition caused by the queued unit mission, visual/audio/order effects, reply codes, and ordering relative to `0x16`/`0x18`.
**Non-Scope:** Full radio protocol, full `0x18` contact lifecycle, full `Mission_Harvest`, complete cargo-credit formulas beyond state-3 gates, Yuri slave miner refinery semantics, and general docking search.
**Confidence:** High for stock receiver branch, queued mission writes, mission `0x10` unload-start writes/order, negative facts, and current Rust handoff. Medium for exact first rendered frame/audio routing outside the verified no-direct-call ranges.
**Active in YR:** Yes. Stock `[GAREFN]`/`[NAREFN]` have `DockUnload=yes`; stock `[CMIN]`/`[HARV]` have `Harvester=yes` and dock to those refineries.

## 1. Overview

Radio `0x15` is a building-side receiver event. On the stock refinery `DockUnload=yes` path it does not itself start unloading. It queues sender mission `0x10` with `commence_now=0`, returns `1`, and leaves cargo, unload visuals, pad/link state, sound, and building animation untouched in that receiver branch.

The real unload side effects start later when the unit's queued mission `0x10` dispatches into `UnitClass::Mission_Deploy_Building`. That path gates on path/facing state; only after the gate accepts does it write the unload-active byte/timer cluster and state `3`, then later state `3` drains storage slots and state `4` clears the unload latch.

This reswarm verifies and consolidates the newer `RADIO_0X15_START_UNLOAD_SIDE_EFFECTS_GHIDRA_REPORT.md` findings. It also confirms the older `miner/BUILDING_RECEIVE_RADIO_0X15_DOCKUNLOAD_HANDOFF_GHIDRA_REPORT.md` was correct but narrower: it proved `0x15` queues mission `0x10` and does not write `+0x2E4`, while this report keeps the follow-on mission side effects in the same handoff.

## 2. Class Layout / Key Offsets

| Offset / slot | Owner | Verified role | Evidence | Active in YR |
|---|---:|---|---|---|
| `+0x16B3` | BuildingType | `DockUnload=yes`; selects stock refinery `0x15` branch | `0x0043C788..0x0043C790`; `rulesmd.ini:11726`, `12519` | Yes |
| `+0x16AE/+0x16AF` | BuildingType | non-stock `0x15` early `return 1` branches before DockUnload | `BuildingClass::Receive_Radio` decompile | Conditional |
| `+0x16A9/+0x16AA/+0x16C1/+0x16C2` | BuildingType | non-stock repair/reload/hospital/armory branch; sets building `+0x6DD`, queues building `0x14`, queues sender `0` | decompile `0x0043C2D0` | Conditional; not stock refinery |
| `+0x16AB` | BuildingType | bunker branch; sets building `+0x6DD`, queues building `0x14` | decompile `0x0043C2D0` | Conditional; not stock refinery |
| `+0x1E8` | MissionClass vtable | `Queue_Mission`; stock `0x15` calls sender `+0x1E8(0x10,0)` | `0x0043C79A..0x0043C7A0` | Yes |
| `+0xB4` | MissionClass | queued mission field | `0x005B3614` | Yes |
| `+0xB8` | MissionClass | mission-init/queued byte cleared by queue write | `0x005B361A` | Yes |
| `+0x388` | UnitClass | facing `RateTimer` sampled before unload-start latch | `0x0073DF56..0x0073DF72` | Yes |
| `+0x674` | UnitClass | locomotor pointer; vtable `+0x4C(0x4000)` called when facing not ready | `0x0073DF7A..0x0073DFAD` | Yes |
| `+0xF8` | UnitClass | unload dump accumulator; zeroed at unload start and later compared against dump threshold | `0x0073DFD0`, `0x0073E35B` | Yes |
| `+0x100/+0x104/+0x108/+0x10C` | UnitClass | unload timer cluster initialized at accepted unload start | `0x0073DFE0..0x0073DFFC` | Yes |
| `+0x6D1` | UnitClass | unload-active latch; set at mission `0x10` unload start, cleared in state `4` | set `0x0073DFDA`; clear `0x0073E1F6` | Yes |
| `+0xBC` | Unit/MissionClass | deploy-building substate; accepted unload start writes `3`; empty gate writes `4` | `0x0073E093`; prior state-3 report | Yes |
| `+0xE0E` | UnitType | `Harvester=yes` gate for stock HARV/CMIN branch | `0x0073E005`; `rulesmd.ini:7364`, `8228` | Yes |
| `+0xE0F` | UnitType | `Weeder=yes`; not stock HARV/CMIN | `0x0073E0BE`; stock INI absence | No for stock path |
| `+0x57C` | BuildingClass | anim slot 8 / `ProductionAnim`; state-4 wait guard | `0x0073E1DF..0x0073E1EA` | Conditional; stock GAREFN/NAREFN normally no active slot-8 wait |
| `+0x584` | BuildingClass | anim slot 10 / `SpecialAnim`; state-3 deposit/clear side effect | prior state-3 report; artmd `SpecialAnim` lines | Yes |

## 3. Core Logic

### 3.1 Ordering relative to `0x16` and `0x18`

The stock refinery admission burst is:

1. `BuildingClass::Receive_Radio(0x0E)` sends `0x12` and requires reply `0x14`.
2. It sends `0x18` before `0x16`; sibling slot owns the `0x18` field lifecycle.
3. `UnitClass::Receive_Radio(0x16)` can later send `0x15` only after its gates pass: not moving, destination is a building, `Unit+0x418` contact flag is set, and current mission is `7`.
4. The destination building receives `0x15` and queues mission `0x10` on the sender.

Evidence: `UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_SCHEDULING_GHIDRA_REPORT.md`; `RADIO_0X18_CONTACT_FLAG_LIFECYCLE_GHIDRA_REPORT.md`; fresh decompile of `BuildingClass::Receive_Radio @ 0x0043C2D0`.

Important split: `0x18` establishes contact state; `0x16` may request `0x15`; `0x15` queues mission `0x10`; mission `0x10` later starts unload-active state.

### 3.2 Stock `0x15` receiver branch

Fresh decompile and assembly agree on the stock DockUnload branch:

```text
if building_current_mission == 0x13:
    return 10
if UnitAbsorb or InfAbsorb:
    return 1
if repair/reload/hospital/armory:
    building+0x6DD = 1
    building.Queue_Mission(0x14, 0)
    sender.Queue_Mission(0, 0)
    return 1
if bunker:
    building+0x6DD = 1
    building.Queue_Mission(0x14, 0)
    return 1
if DockUnload:
    sender.Queue_Mission(0x10, 0)
    return 1
fall through to TechnoClass::Receive_Radio
```

The exact stock block is:

```asm
0043C788  MOV  CL, byte ptr [EAX + 0x16B3]
0043C78E  TEST CL, CL
0043C790  JZ   0x0043CE43
0043C796  MOV  ECX, dword ptr [ESP + 0x54]   ; sender
0043C79A  PUSH 0x0                            ; commence_now = 0
0043C79C  PUSH 0x10                           ; queued mission
0043C79E  MOV  EDX, dword ptr [ECX]
0043C7A0  CALL dword ptr [EDX + 0x1E8]
0043C7A9  MOV  EAX, 0x1
0043C7B2  RET  0xC
```

Immediate stock side effects are therefore limited to the sender virtual call and reply `1`.

### 3.3 Queue_Mission effects

`MissionClass::Queue_Mission(0x10,0)` is not an immediate dispatch:

- Reads current mission `+0xAC` at `0x005B35E7`.
- Blocks some protected current missions (`0x13`, and current `0x1C` with requested `5`).
- If the requested mission is not redundant, writes `+0xB4 = 0x10` at `0x005B3614`.
- Clears byte `+0xB8 = 0` at `0x005B361A`.
- Reads the third argument at `0x005B3621`; stock `0x15` passed `0`, so `0x005B3627` jumps over vtable `+0x200` / `+0x1EC` commence calls.

So `0x15` queues work for a future mission tick. It does not start the mission in the receiver call.

### 3.4 Mission `0x10` start-unload gates

When mission `0x10` dispatches to `UnitClass::Mission_Deploy_Building`, stock harvester unload-start passes through these gates:

- `PathType::Has_Valid_Steps` gate at `0x0073DEE0..0x0073DEE9`; false path returns/cleans up before unload latch writes.
- Facing `RateTimer` gate at `0x0073DF56..0x0073DF72`, accepting only `((current >> 7) + 1) & 0x1FE == 0x80`.
- If not accepted and `Unit+0x6AF` is clear, call locomotor `+0x4C(0x4000)` and return delay `5` without setting `+0x6D1` or state `3`. Evidence: `0x0073DF7A..0x0073DFBC`.
- If already accepted, proceed to unload-start writes.

This means an implementation must not turn radio `0x15` into cargo drain, visual swap, pad occupancy, or unload-active state immediately.

### 3.5 Accepted mission `0x10` unload-start write order

When facing/path gates pass and `+0x6D1` is not already set, the verified write order is:

1. `0x0073DFD0`: `Unit+0xF8 = 0`.
2. `0x0073DFDA`: `Unit+0x6D1 = 1`.
3. `0x0073DFE0`: read global current frame `0x00A8ED84`.
4. `0x0073DFED`: `Unit+0x10C = 1`.
5. `0x0073DFF3`: `Unit+0x100 = current frame`.
6. `0x0073DFF5..0x0073DFF9`: `Unit+0x104 = stack value`.
7. `0x0073DFFC`: `Unit+0x108 = 1`.
8. `0x0073E005..0x0073E00D`: check `UnitType+0xE0E Harvester=yes`.
9. `0x0073E013..0x0073E08E`: if harvester branch finds the adjacent building, call `BuildingClass::SetAnimSlotImage(slot 7, damaged, 0)`.
10. `0x0073E093`: `Unit+0xBC = 3`.
11. `0x0073E09D`: jump to mission-timer epilogue.

Negative details:

- No coordinate snap.
- No `+0x2E4` link write.
- No `+0x418` contact write.
- No direct `VocClass`/sound call in the verified receiver branch or unload-start init block.
- No cargo drain in the unload-start block.
- No explicit facing-byte snap; the path requires the facing timer already be inside the accepted east window.

### 3.6 State-3 drain and state-4 cleanup are later

State `3` only considers storage once the dump threshold is met:

- It compares `Unit+0xF8` against `RulesClass+0x1528 HarvesterDumpRate * 900.0`. Evidence: `0x0073E35B..0x0073E374`.
- Stock default is `0.016 * 900.0 = 14.4` frames.
- On a real slot, it removes the full first non-empty storage slot, credits the refinery owner, applies purifier bonus, and resets `+0xF8 = 0`.
- When `FindFirstNonEmptySlot == -1` on a later gate, state `3` writes state `4`, handles anim slot 8/10 side effects, and returns `1`.
- State `4` later clears `Unit+0x6D1 = 0` at `0x0073E1F6`, after a `Refinery=yes && building+0x57C != 0` wait guard. Stock GAREFN/NAREFN do not normally define active `ProductionAnim` slot 8.

## 4. INI Keys

| INI key | Stock value / source | Effect | Active in YR |
|---|---|---|---|
| `[GAREFN] DockUnload` | `yes`, `rulesmd.ini:11726` | selects stock building `0x15` branch | Yes |
| `[NAREFN] DockUnload` | `yes`, `rulesmd.ini:12519` | same | Yes |
| `[GAREFN]/[NAREFN] Refinery` | `yes`, `rulesmd.ini:11727`, `12520` | later state-3/state-4 refinery branches | Yes |
| `[GAREFN]/[NAREFN] NumberOfDocks` | `1`, `rulesmd.ini:11729`, `12521` | contact/admission capacity, not direct `0x15` side effect | Yes |
| `[CMIN] Harvester` | `yes`, `rulesmd.ini:7364` | reaches UnitType `+0xE0E` harvester unload path | Yes |
| `[HARV] Harvester` | `yes`, `rulesmd.ini:8228` | same | Yes |
| `[CMIN] Storage` | `20`, `rulesmd.ini:7374` | later storage amount | Yes |
| `[HARV] Storage` | `40`, `rulesmd.ini:8236` | later storage amount | Yes |
| `[CMIN] UnloadingClass` | `CMON`, `rulesmd.ini:7384` | render swap while `+0x6D1` is live | Yes |
| `[HARV] UnloadingClass` | `HORV`, `rulesmd.ini:8246` | render swap while `+0x6D1` is live | Yes |
| `[General] PurifierBonus` | `.25`, `rulesmd.ini:340` | later slot-drain bonus calculation | Yes |
| `[GAREFN]/[NAREFN] SpecialAnim` | `GAREFNOR` / `NAREFNOR`, `artmd.ini:1787`, `1739` | later deposit animation slot, not immediate `0x15` effect | Yes |

## 5. Integration Points

| Function / point | Role | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass::Receive_Radio(0x0E)` | sends `0x18` then `0x16` after `0x12 == 0x14` | prior `0x16` scheduling report | Yes |
| `UnitClass::Receive_Radio(0x16)` | gated sender of `0x15` to destination building | `0x0073771B..0x00737780` in prior report | Yes |
| `UnitClass::PerCellProcess` | alternate adjacent-building `0x15` sender after `+0x418` contact | `RADIO_0X18_CONTACT_FLAG_LIFECYCLE_GHIDRA_REPORT.md` | Conditional |
| `BuildingClass::Receive_Radio(0x15)` | stock receiver; queues sender mission `0x10`, returns `1` | decompile and assembly `0x0043C788..0x0043C7B2` | Yes |
| `MissionClass::Queue_Mission` | writes queued mission fields; stock call does not commence | decompile and assembly `0x005B35E0..0x005B3644` | Yes |
| `MissionClass::Mission_Dispatch` | later dispatches mission `0x10` through UnitClass vtable `+0x23C` | prior report; `0x005B3060` | Yes |
| `UnitClass::Mission_Deploy_Building` | owns unload-start latch/state, cargo gates, and state-4 clear | decompile and assembly `0x0073DEE0..0x0073E545` | Yes |

## 6. Current Rust Implementation Status

Current Rust has already absorbed part of this correction:

| Rust surface | Current behavior observed | Parity status |
|---|---|---|
| `src/sim/miner/mod.rs:100..126` | `MissionEnter`, `FaceSync`, `MissionQueued`, `Pivoting`, `Unloading`, `Departing` are split and documented with `0x15` not being unload start | Directionally aligned |
| `src/sim/miner/miner_dock_sequence.rs:917..946` | `phase_face_sync` marks contact-entered, syncs facing, waits for Enter retry, then enters `MissionQueued` | Broadly aligned with `0x18/0x16` split; exact mission timing still representation-level |
| `src/sim/miner/miner_dock_sequence.rs:948..950` | `phase_mission_queued` advances to `Pivoting` | Partial; it represents queued mission `0x10` but not exact MissionClass `+0xAC/+0xB4/+0xB8` dispatch fields |
| `src/sim/miner/miner_dock_sequence.rs:1000..1030` | `phase_pivoting` waits for mission deploy due, samples facing, schedules five-frame wait when not accepted | Much closer to binary; still no exact `PathType::Has_Valid_Steps` cleanup gate here |
| `src/sim/miner/miner_dock_sequence.rs:977..994` | `start_unload_deploy` sets `display_type_override`, `unload_active`, `unload_accumulator=0`, timer cluster analogs, and enters `Unloading` | Close in intent but not byte-equivalent: no explicit `+0xF8/+0x100..+0x10C/+0xBC` struct fields, no building slot-7 anim side effect |
| `src/sim/miner/miner_dock_sequence.rs:1033..1141` | `phase_unloading` drains one resource slot per threshold and delays empty release until the empty gate | Aligned with state-3 slot concept; mechanism still Rust-side accumulator rather than Techno timer cluster |
| `src/sim/miner/miner_dock.rs:22..36`, `:92..107` | `contact_entered` is separate from `on_pad`; `on_pad` still exists as physical/internal bookkeeping | Risk: stock zero-link unload has no proven `+0x2E4`/physical on-pad field equivalent |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Prior exact `0x15` report status | verified | `RADIO_0X15_START_UNLOAD_SIDE_EFFECTS_GHIDRA_REPORT.md` | none; this reswarm consolidates it |
| Stock `BuildingClass::Receive_Radio(0x15)` branch | verified | decompile `0x0043C2D0`; assembly `0x0043C788..0x0043C7B2` | none |
| Non-stock repair/reload/bunker/hospital/armory `0x15` branches | touched-not-exhausted | decompile `0x0043C2D0` | out-of-scope for stock refinery unload |
| `Queue_Mission(0x10,0)` writes and no immediate commence | verified | decompile and assembly `0x005B35E0..0x005B3644` | none |
| `0x16 -> 0x15` ordering | verified via sibling doc | `UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_SCHEDULING_GHIDRA_REPORT.md` | full `0x18` owned by slot 2 |
| Mission `0x10` unload-start gates | verified | `0x0073DEE0..0x0073DFBC` | exact PathType helper internals out-of-scope |
| Accepted unload-start write order | verified | `0x0073DFD0..0x0073E093` | exact `+0x104` meaning not needed for this handoff |
| Later state-3 cargo gate | verified for role/order | `0x0073E35B..0x0073E374`; state-3 timing doc | full credit arithmetic belongs to ore deposit reports |
| State-4 `+0x6D1` clear | verified | `0x0073E1D5..0x0073E1F6` | exact rendered frame requires runtime capture |
| Direct sound call in stock `0x15` receiver / unload-start block | verified negative | no sound call in checked ranges | full global audio routing remains outside slice |
| Current Rust surfaces | verified scan | source ranges listed in section 6 | implementation patch separate |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-15-001 - Which function receives stock refinery radio 0x15? -> The destination building receives it in BuildingClass::Receive_Radio case 0x15.` (evidence: `0x0043C2D0`; Active in YR: Yes)
- `[RESOLVED] OQ-15-002 - What immediate side effect does the stock DockUnload branch perform? -> It calls sender vtable +0x1E8 with mission 0x10 and commence flag 0, then returns 1.` (evidence: `0x0043C788..0x0043C7B2`; Active in YR: Yes)
- `[RESOLVED] OQ-15-003 - Does stock 0x15 write +0x2E4, +0x418, +0x6D1, or coordinates? -> No in the verified stock receiver block.` (evidence: `0x0043C788..0x0043C7B2`; Active in YR: Yes)
- `[RESOLVED] OQ-15-004 - Does Queue_Mission(0x10,0) immediately dispatch mission 0x10? -> No; flag 0 skips the commence calls after writing +0xB4/+0xB8 when not redundant.` (evidence: `0x005B35E0..0x005B3644`; Active in YR: Yes)
- `[RESOLVED] OQ-15-005 - Where do unload-active side effects begin? -> Later mission 0x10 dispatch in UnitClass::Mission_Deploy_Building, after path/facing gates.` (evidence: `0x0073DEE0..0x0073E093`; Active in YR: Yes)
- `[RESOLVED] OQ-15-006 - What happens if facing is not ready? -> Locomotor +0x4C(0x4000) is called and mission returns 5 without setting unload-active state.` (evidence: `0x0073DF56..0x0073DFBC`; Active in YR: Yes)
- `[RESOLVED] OQ-15-007 - What exact fields are written at accepted unload start? -> +0xF8=0, +0x6D1=1, +0x100=current frame, +0x104=stack value, +0x108=1, +0x10C=1, optional building slot 7, then +0xBC=3.` (evidence: `0x0073DFD0..0x0073E093`; Active in YR: Yes)
- `[RESOLVED] OQ-15-008 - Does 0x15 or unload-start directly drain cargo? -> No; state 3 later checks +0xF8 against HarvesterDumpRate*900 before draining.` (evidence: `0x0073E35B..0x0073E374`; Active in YR: Yes)
- `[RESOLVED] OQ-15-009 - Does stock 0x15 emit a deploy/dock sound directly? -> No direct sound call in stock receiver block; no direct sound call in the verified unload-start init block.` (evidence: `0x0043C788..0x0043C7B2`, `0x0073DFD0..0x0073E093`; Active in YR: Yes for negative scoped to these blocks)
- `[RESOLVED] OQ-15-010 - What reply code does stock 0x15 return? -> `1` after queueing mission 0x10; if the building is currently mission 0x13 it returns `10` before DockUnload.` (evidence: decompile `0x0043C2D0`; assembly `0x0043C7A9`; Active in YR: Yes)
- `[RESOLVED] OQ-15-011 - Is the path active for stock YR? -> Yes for stock CMIN/HARV docking with GAREFN/NAREFN.` (evidence: `rulesmd.ini:7364`, `8228`, `11726`, `12519`)
- `[DEFERRED] OQ-15-012 - Exact first rendered frame for CMON/HORV after +0x6D1 set.` (category: `needs-runtime-debugger`; reason: static evidence proves causal latch; frame presentation requires runtime/render trace; next-step-if-pursued: trace `+0x6D1` and draw type across unload-start tick)
- `[DEFERRED] OQ-15-013 - Exact global audio path, if any, around unload start beyond checked ranges.` (category: `requires-different-system-context`; reason: no direct call appears in receiver/init blocks, but global audio/event routing is a separate subsystem; next-step-if-pursued: trace sound event emitters around refinery dock/deposit)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock `0x15` only queues sender mission `0x10` and returns `1` | `0x0043C788..0x0043C7B2` | mostly represented by `MissionQueued`; exact MissionClass queued fields absent | `src/sim/miner/mod.rs`, `src/sim/miner/miner_dock_sequence.rs::phase_mission_queued` | Preserve `0x15` as queued-mission boundary with no unload, cargo, pad, sound, or anim side effects | `radio_0x15_queues_mission_0x10_without_unload_side_effects` | Do not re-collapse `0x15` and unload start into one transition |
| `Queue_Mission(0x10,0)` writes `+0xB4/+0xB8` only when non-redundant and does not commence | `0x005B35E0..0x005B3644` | Rust phase state does not model current/queued mission redundancy/protected mission checks | miner mission scheduler / dock FSM | If exact byte parity is pursued, model queued/current mission distinction and protected current mission guards | `redundant_radio_0x15_does_not_rewrite_or_commence_mission` | Do not assume every `0x15` necessarily changes mission state |
| Mission `0x10` path/facing gates precede unload latch writes; not-ready returns delay `5` | `0x0073DEE0..0x0073DFBC` | Rust has facing wait delay but lacks exact PathType cleanup branch | `phase_pivoting`, future deploy-building helper | Add/prove exact path-valid cleanup before setting unload-active state; keep five-frame retry cadence | `mission_0x10_not_facing_ready_returns_wait_without_unload_active` | Do not poll the deploy gate every tick or bypass path cleanup |
| Accepted unload start writes `+0xF8`, `+0x6D1`, timer cluster, optional slot 7, then state `3` | `0x0073DFD0..0x0073E093` | Rust has analogous fields but not byte-exact layout/order and no building slot-7 side effect | `start_unload_deploy`, building animation system | Treat `unload_active` as the `+0x6D1` equivalent, keep write/order semantics, and add/prove slot-7 anim trigger if needed | `accepted_mission_0x10_sets_unload_latch_before_state3_and_before_first_drain` | Do not use display override alone as the simulation source of truth |
| First cargo drain is later state-3 threshold, not radio/unload-start frame | `0x0073E35B..0x0073E374` | Rust slot-drain concept is close, but uses Rust accumulator rather than Techno timer cluster | `phase_unloading`, `Miner` timer fields | Preserve no-drain-on-start and one slot per threshold crossing | `first_slot_drain_waits_for_dump_rate_after_unload_start` | Do not credit cargo on the radio handoff or first unload-start tick |
| State `4` clears `+0x6D1`; stock `0x15` and state `3` empty gate do not clear it directly | `0x0073E1D5..0x0073E1F6`; state-3 timing doc | Rust clears display/contact in `phase_departing`; this is directionally right | `phase_departing`, render display override | Keep unload visual/latch live until state-4 handoff, not last real slot drain | `unloading_class_clears_on_state4_handoff_not_last_slot_drain` | Do not clear CMON/HORV when cargo first becomes empty |

## 10. Negative Facts / Do Not Do

- Do not implement radio `0x15` as a physical pad snap, coordinate change, or `GetDockCoord` move.
- Do not write stock reciprocal `+0x2E4` links in `0x15` or unload-start for normal GAREFN/NAREFN unload.
- Do not start cargo drain, set `+0x6D1`, set display override, or play dock sound in the building receiver's stock `0x15` branch.
- Do not use `UnitClass::Mission_Unload @ 0x00740EF0` as the stock refinery path; stock `0x15` queues mission `0x10`, which dispatches to `UnitClass::Mission_Deploy_Building @ 0x0073D630`.
- Do not treat `0x18` contact establishment as unload start; `0x18`, `0x16`, `0x15`, and mission `0x10` are distinct steps.
- Do not snap facing on accepted unload start; the binary accepts the live facing timer window and then writes unload state.
- Do not use `SpecialAnim` duration as a post-empty unload cooldown; the stock state-4 guard checks slot 8 `ProductionAnim`, not slot 10 `SpecialAnim`.

## 11. Remaining Uncertainty

- Exact first rendered frame where `CMON`/`HORV` appears after `+0x6D1` is set remains runtime/render-trace work.
- Exact global audio routing around unload start remains outside this slot. Static checks prove no direct sound call in the stock `0x15` receiver or accepted unload-start init block.
- Exact `+0x104` semantic label is still not needed for the `0x15` handoff, though it should be named if a future byte-field implementation models the full timer cluster.

## Sources

- Fresh Ghidra decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`.
- Fresh Ghidra decompile: `MissionClass::Queue_Mission @ 0x005B35E0`.
- Fresh Ghidra decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`.
- Fresh Ghidra assembly context: `0x0043C788`, `0x0043C796`, `0x0043C7A0`.
- Fresh Ghidra assembly context: `0x005B35E7`, `0x005B3614`, `0x005B361A`, `0x005B3621`.
- Fresh Ghidra assembly context: `0x0073DF56`, `0x0073DF7A`, `0x0073DFD0`, `0x0073DFDA`, `0x0073DFE0`, `0x0073DFFC`, `0x0073E093`.
- Fresh Ghidra assembly context: `0x0073E35B`, `0x0073E374`, `0x0073E4A9`.
- Fresh Ghidra assembly context: `0x0073E1D5`, `0x0073E1F6`, `0x0073E207`, `0x0073E26F`.
- Prior docs: `RADIO_0X15_START_UNLOAD_SIDE_EFFECTS_GHIDRA_REPORT.md`, `miner/BUILDING_RECEIVE_RADIO_0X15_DOCKUNLOAD_HANDOFF_GHIDRA_REPORT.md`, `UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_SCHEDULING_GHIDRA_REPORT.md`, `RADIO_0X18_CONTACT_FLAG_LIFECYCLE_GHIDRA_REPORT.md`, `UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md`, `miner/MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_STATE3_STATE4_TIMING_GHIDRA_REPORT.md`, `miner/TECHNOCLASS_AI_UPDATE_UNLOAD_ACCUMULATOR_ORDERING_GHIDRA_REPORT.md`.
- INI/art checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scanned: `src/sim/miner/mod.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_tests.rs`.
