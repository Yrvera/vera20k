# Radio 0x15 Start-Unload Side Effects - Ghidra Research Report

**Address(es):** `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x15` (`0x0043C6F2..0x0043C7B2`), `MissionClass::Queue_Mission @ 0x005B35E0`, `MissionClass::Mission_Dispatch @ 0x005B3060`, `UnitClass::Mission_Deploy_Building @ 0x0073D630`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** radio `0x15` receiver/start-unload side effects in the stock `HARV/CMIN -> GAREFN/NAREFN` refinery docking path, plus the first mission-dispatch effects that turn the queued mission into the unload latch/state.  
**Non-Scope:** accepted-cell vs GetDockCoord coordinate proof, multi-miner queue ordering, full post-empty departure, and complete cargo-credit arithmetic beyond the first unload gate.  
**Confidence:** High for receiver branch, fields written, mission queueing, first unload latch/state, negative facts, and Rust handoff deltas.  
**Active in YR:** Yes. Stock `[GAREFN]`/`[NAREFN]` set `DockUnload=yes`; stock `[CMIN]`/`[HARV]` set `Harvester=yes` and reach `UnitClass::Mission_Deploy_Building`.

## 0. Scope Contract

**Target question:** What does radio `0x15` actually do in the miner/refinery docking path: receiver function, fields written on unit/building, mission/state/timer changes, cargo/unload timer start, direct sound/animation side effects, and how should that map to Rust `phase_linked` / `Pivoting` / `Unloading`?

**Non-goals:** Do not re-investigate settled coordinate split; do not decode all `Mission_Deploy_Building` passenger/deploy branches; do not decide which `0x15` sender wins first; do not patch Rust.

**Evidence needed to mark COMPLETE:** Decompile plus assembly for `BuildingClass::Receive_Radio(0x15)`, `MissionClass::Queue_Mission`, `MissionClass::Mission_Dispatch` mission `0x10`, and `UnitClass::Mission_Deploy_Building` first stock harvester unload entry; INI/default proof for stock activation; Rust surface scan around `phase_linked`, `Pivoting`, and `Unloading`.

**Stop conditions:** Stop once the stock `DockUnload=yes` branch is isolated, every immediate field write is known, the first dispatch-side unload latch/state is mapped, and all remaining non-stock receiver branches are marked out of scope or conditional.

## 1. Overview

Radio `0x15` is received by the destination building. In the stock refinery path, `BuildingClass::Receive_Radio(0x15)` does not snap the miner, does not set a reciprocal dock pointer, does not start cargo draining, and does not play dock sound/animation directly. Its stock `DockUnload=yes` side effect is one call to the sender unit's mission queue slot with mission `0x10`, queued flag `0`, then return `1`.

The actual stock unload FSM starts later when mission `0x10` dispatches through `UnitClass::Mission_Deploy_Building @ 0x0073D630`. That function performs the facing/rate gate, sets `Unit+0x6D1`, zeros `Unit+0xF8`, initializes timer fields `+0x100..+0x10C`, optionally opens refinery anim slot `7`, and writes mission substate `Unit+0xBC = 3`.

## 2. Class Layout / Key Offsets

| Offset / slot | Owner | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| BuildingType `+0x16B3` | BuildingType | `DockUnload=yes`; selects stock refinery `0x15` branch | `0x0043C788..0x0043C7A6`; `rulesmd.ini:11726`, `12519` | Yes |
| BuildingType `+0x16AE/+0x16AF` | BuildingType | early return `1` in `0x15`; non-stock repair/reload style branches | `0x0043C710..0x0043C72C` | Conditional |
| BuildingType `+0x16A9/+0x16AA/+0x16C1/+0x16C2` | BuildingType | non-stock receiver branch; writes building `+0x6DD`, queues building mission `0x14`, queues sender mission `0` | `0x0043C732..0x0043C7DA` | Conditional |
| BuildingType `+0x16AB` | BuildingType | non-stock receiver branch; writes building `+0x6DD`, queues building mission `0x14` | `0x0043C75A..0x0043C785` | Conditional |
| Building byte `+0x6DD` | Building | set only in non-stock `0x15` branches, not stock `DockUnload` | `0x0043C76C`, `0x0043C7BD` | Conditional |
| Unit vtable `+0x1E8` | Unit/MissionClass | `MissionClass::Queue_Mission` | stock call `0x0043C796..0x0043C7A0`; vtable entry `0x007F5E58 -> 0x005B35E0` | Yes |
| Unit mission id `0x10` | Unit | queued by stock refinery `0x15`; dispatched through Unit vtable `+0x23C` | `0x0043C79A..0x0043C7A0`; `0x005B321D`; vtable entry `0x007F5EAC -> 0x0073D630` | Yes |
| Unit `+0xB4` | MissionClass | queued mission field | `0x005B3614` | Yes |
| Unit `+0xB8` | MissionClass | mission-just-started/init byte reset on queue write | `0x005B361A` | Yes |
| Unit `+0xBC` | MissionClass | mission substate; stock unload writes `3` after init | `0x0073E093` | Yes |
| Unit `+0xF8` | Unit | dump-rate accumulator; zeroed on first unload init | `0x0073DFD0` | Yes |
| Unit `+0x100..+0x10C` | Unit | timer fields initialized when unload latch starts | `0x0073DFE0..0x0073DFFC` | Yes |
| Unit byte `+0x6D1` | Unit | unload-active latch; set on first unload init, not by radio `0x15` itself | `0x0073DFDA` | Yes |
| UnitType `+0xE0E` | UnitType | `Harvester=yes` gate for stock HARV/CMIN unload branch | `0x0073D678`; `rulesmd.ini:7314`, `7364`, `8184`, `8228` | Yes |
| UnitType `+0xE0F` | UnitType | `Weeder=yes`; stock HARV/CMIN do not set it | `0x0073D686`; stock INI absence | No for stock HARV/CMIN |
| `DAT_0089F6A0/2` | global | adjacent refinery lookup `(-1,0)` in prior report; runtime-initialized table | `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_REACHABILITY_GHIDRA_REPORT` | Yes |

## 3. Core Logic

### 3.1 Switch mapping proves `param_3 == 0x15` reaches `0x0043C6F2`

At `0x0043C2DD..0x0043C2F1`, `BuildingClass::Receive_Radio` subtracts `3` from the incoming radio id, bounds-checks against `0x12`, loads a byte from `0x0043CE88 + (radio - 3)`, then jumps through the table at `0x0043CE60`. The lookup byte for index `0x12` (`0x15 - 3`) is `8`, and table entry `8` is `0x0043C6F2`.

### 3.2 Stock `DockUnload=yes` receiver branch

Verified stock branch:

1. `0x0043C6F2..0x0043C6FF`: call receiver building vtable `+0x184`; if mission is `0x13`, return `10`.
2. `0x0043C710..0x0043C72C`: if type `+0x16AE` or `+0x16AF` is set, return `1`.
3. `0x0043C732..0x0043C758`: test non-stock flags `+0x16A9`, `+0x16AA`, `+0x16C1`, `+0x16C2`.
4. `0x0043C75A..0x0043C785`: if `+0x16AB` is set, set building `+0x6DD = 1`, queue building mission `0x14`, return `1`.
5. `0x0043C788..0x0043C7A6`: if `+0x16B3 DockUnload=yes` is set, call sender unit vtable `+0x1E8(0x10, 0)`, then return `1`.
6. If none match, fall through to base `TechnoClass::Receive_Radio`.

For stock `GAREFN/NAREFN`, `+0x16B3` is true and the repair/reload/helipad/non-stock branches are false, so the only stock side effect is `Queue_Mission(0x10, 0)` on the unit sender.

### 3.3 Immediate negative facts for stock `0x15`

Inside the stock `+0x16B3` block (`0x0043C788..0x0043C7B2`):

- no write to building `+0x6DD`;
- no write to unit or building `+0x2E4`;
- no write to unit `+0x6D1`, `+0xF8`, or `+0xBC`;
- no coordinate read/write and no `GetDockCoord`;
- no sound call;
- no `SetAnimSlotImage` / `ClearAnimSlot`;
- no cargo/storage function call;
- return value is hard `1`; the queued mission's later success is not reflected in this return.

### 3.4 `MissionClass::Queue_Mission(0x10, 0)` side effects

`MissionClass::Queue_Mission @ 0x005B35E0` writes only if the new mission is not redundant:

- `0x005B35E7`: read current mission `+0xAC`.
- `0x005B35ED..0x005B35FA`: block if current mission is `0x13`, or current mission is `0x1C` and requested mission is `5`.
- `0x005B35FC..0x005B3612`: skip write if requested mission is `-1`, or if current mission and queued mission make it redundant.
- `0x005B3614`: write queued mission `+0xB4 = requested mission`.
- `0x005B361A`: write byte `+0xB8 = 0`.
- `0x005B3621..0x005B3641`: only if the queued flag argument is nonzero, call vtable `+0x200` and then vtable `+0x1EC`.

The stock `0x15` call passes queued flag `0`, so it does not commence the queued mission inside the radio receiver.

### 3.5 Mission `0x10` dispatch target for UnitClass

`MissionClass::Mission_Dispatch @ 0x005B3060` dispatches current mission `0x10` through vtable slot `+0x23C`. The UnitClass vtable entry at `0x007F5EAC` points to `0x0073D630`, so mission `0x10` for stock unit harvesters reaches `UnitClass::Mission_Deploy_Building`.

This corrects a common shorthand: `0x10` in this path is not `UnitClass::Mission_Unload @ 0x00740EF0`; `0x00740EF0` is at UnitClass vtable slot `+0x24C` and is dispatched by mission `0x14`.

### 3.6 First stock unload init inside `UnitClass::Mission_Deploy_Building`

The stock HARV/CMIN path reaches the harvester block because `UnitType+0xE0E Harvester=yes` is set (`0x0073D678`) and `+0xE0F Weeder` is not set (`0x0073D686`). In the valid-path/facing-aligned branch:

1. `0x0073DEE0..0x0073DEE9`: `PathType::Has_Valid_Steps()` must be true to proceed toward rate/facing/state dispatch. False goes to cleanup.
2. `0x0073DF56..0x0073DF78`: read `RateTimer::Current(+0x388)`, compute `((current >> 7) + 1) & 0x1FE`, and require `0x80`.
3. `0x0073DF7A..0x0073DFBC`: if not facing-ready and not chrono/teleport flag `+0x6AF`, call locomotor vtable `+0x4C(0x4000)` and return `5`.
4. `0x0073DFBD..0x0073DFC5`: if `+0x6D1` is already set, skip init and continue state processing.
5. `0x0073DFD0`: set `Unit+0xF8 = 0`.
6. `0x0073DFDA`: set byte `Unit+0x6D1 = 1`.
7. `0x0073DFE0..0x0073DFFC`: initialize timer fields at `+0x100..+0x10C` using `g_CurrentFrameCounter` and duration `1`.
8. `0x0073E013..0x0073E08E`: for `Harvester=yes`, find the adjacent refinery and call `BuildingClass::SetAnimSlotImage(7, damaged, 0)` if found.
9. `0x0073E093`: set `Unit+0xBC = 3`.
10. `0x0073E289..0x0073E2BE`: return mission timer entry delay plus `RandomRanged(0,2)` through the epilogue.

Cargo draining is not started by radio `0x15` directly; the first actual dump gate is later in state `3`, comparing `RulesClass+0x1528 HarvesterDumpRate * 900.0` against `Unit+0xF8` (`0x0073E35B..0x0073E374`).

## 4. INI Keys

| INI key | Stock value | Effect in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `[GAREFN] DockUnload` | `yes` | enables `BuildingClass::Receive_Radio(0x15)` stock branch | `rulesmd.ini:11726`; `0x0043C788..0x0043C7A6` | Yes |
| `[NAREFN] DockUnload` | `yes` | same | `rulesmd.ini:12519`; `0x0043C788..0x0043C7A6` | Yes |
| `[GAREFN]/[NAREFN] Refinery` | `yes` | later state-4/animation behavior; not immediate `0x15` receiver gate | `rulesmd.ini:11727`, `12520` | Yes |
| `[CMIN] Harvester` | `yes` | reaches `UnitType+0xE0E` harvester unload path | `rulesmd.ini:7314`, `7364`; `0x0073D678` | Yes |
| `[HARV] Harvester` | `yes` | same | `rulesmd.ini:8184`, `8228`; `0x0073D678` | Yes |
| `[CMIN] Storage` | `20` | later state-3 cargo capacity | `rulesmd.ini:7374` | Yes |
| `[HARV] Storage` | `40` | later state-3 cargo capacity | `rulesmd.ini:8236` | Yes |
| `[CMIN]/[HARV] UnloadingClass` | `CMON` / `HORV` | visual class used by draw/render paths while unload-active latch is live; not set by radio `0x15` itself | `rulesmd.ini:7384`, `8246`; existing draw/unload reports | Yes |
| `[General] HarvesterDumpRate` | `0.016` | later dump threshold `0.016 * 900.0 = 14.4` frames | prior reachability report; `0x0073E35B..0x0073E374` | Yes |
| `[General] PurifierBonus` | `.25` | later credit bonus calculation after slot drain | `rulesmd.ini:340`; `0x0073E42B..0x0073E4CE` | Yes |

## 5. Integration Points

| Function / point | Role | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass::Receive_Radio(0x16)` | sender path can transmit `0x15` to destination building | prior slot report; no receiver side effects here | Yes |
| `UnitClass::PerCellProcess @ 0x00739EC0` | alternate sender path can transmit `0x15` | prior slot report | Yes |
| `BuildingClass::Receive_Radio @ 0x0043C2D0` | receiver of `0x15`; stock branch queues mission `0x10` | decompile + assembly `0x0043C6F2..0x0043C7B2` | Yes |
| `MissionClass::Queue_Mission @ 0x005B35E0` | stores queued mission and reset byte; with flag `0` does not commence | decompile + assembly `0x005B35E0..0x005B3642` | Yes |
| `MissionClass::Mission_Dispatch @ 0x005B3060` | dispatches mission `0x10` via vtable `+0x23C` | decompile | Yes |
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | first real unload/pivot/dump latch state machine | decompile + assembly ranges listed above | Yes |

## 6. Current Rust Implementation Status

| Rust surface | Current behavior observed | Binary delta / risk |
|---|---|---|
| `src/sim/miner/miner_dock_sequence.rs:613 phase_mission_enter` | accepted-cell/already-there state can mark contact entered and move to `Linked` | Should represent `0x18/0x16/0x15 -> Queue_Mission(0x10)` boundaries; do not treat contact entry alone as mission `0x10` dispatch |
| `src/sim/miner/miner_dock_sequence.rs:700 phase_linked` | writes snapshot to `pad`, marks `link_on_pad`, sets `display_type_override`, starts pivot, emits `DockDeploy`, moves to `Pivoting` | Stock `0x15` itself does none of these immediate side effects; the first dispatch of mission `0x10` does facing gate/latch/init. Snapshot snap/link/sound are suspect unless separately proven by the mission body/render path |
| `src/sim/miner/miner_dock_sequence.rs:747 phase_pivoting` | drives local `FacingClass` until east; then seeds `unload_timer` and enters `Unloading` | Broadly maps to `Mission_Deploy_Building` facing gate plus state-3 init, but binary sets `+0x6D1`, `+0xF8=0`, timer fields, and `+0xBC=3`; cargo gate comes later |
| `src/sim/miner/miner_dock_sequence.rs:793 phase_unloading` | drains Rust cargo by `unload_timer` intervals and credits owner | Corresponds to later state-3 gate, not radio `0x15` immediate effect |
| `src/sim/miner/mod.rs:86 RefineryDockPhase` docs | `Linked` is described as `0x15 pad-arrival handoff` | Needs wording split: `0x15` queues mission `0x10`; mission dispatch starts pivot/unload latch |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass::Receive_Radio` switch mapping for `0x15` | verified | `0x0043C2DD..0x0043C2F1`, table `0x0043CE60/0x0043CE88` | none |
| Stock `DockUnload=yes` branch side effect | verified | `0x0043C788..0x0043C7A6` | none |
| Non-stock `0x15` branches | touched-not-exhausted | `0x0043C710..0x0043C7DA` | full repair/helipad/passenger semantics out of scope |
| `MissionClass::Queue_Mission(0x10,0)` writes | verified | `0x005B35E0..0x005B3642` | none |
| UnitClass mission `0x10` dispatch target | verified | `0x005B3060`; vtable `0x007F5EAC -> 0x0073D630` | none |
| `UnitClass::Mission_Deploy_Building` first unload init | verified | `0x0073DEE0..0x0073E09D` | none for start-unload slice |
| Later state-3 cargo gate | touched-not-exhausted | `0x0073E35B..0x0073E4D0`; prior reachability report | full credit/purifier arithmetic belongs to existing ore deposit reports |
| Sound side effects in immediate `0x15` stock branch | verified negative | absence in `0x0043C788..0x0043C7A6` | render/audio side effects elsewhere out of scope |
| Rust `phase_linked` / `Pivoting` / `Unloading` mapping | verified scan | source lines listed in section 6 | implementation patch separate |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Which receiver handles stock miner/refinery radio 0x15? -> The destination building receives it through BuildingClass::Receive_Radio case 0x15.` (evidence: `0x0043C2D0`, `0x0043C6F2`)
- `[RESOLVED] OQ-02 - What does stock DockUnload=yes 0x15 write immediately? -> It queues sender mission 0x10 with queued flag 0 and returns 1; no stock building/unit state field is otherwise written in the branch.` (evidence: `0x0043C788..0x0043C7A6`)
- `[RESOLVED] OQ-03 - Does stock 0x15 create reciprocal +0x2E4 pad links? -> No write to unit/building +0x2E4 appears in the stock branch.` (evidence: `0x0043C788..0x0043C7A6`)
- `[RESOLVED] OQ-04 - Does stock 0x15 start cargo draining immediately? -> No; cargo/drain logic appears later in UnitClass::Mission_Deploy_Building state 3.` (evidence: `0x0043C788..0x0043C7A6`, `0x0073E35B..0x0073E4D0`)
- `[RESOLVED] OQ-05 - Does stock 0x15 play DockDeploy or set anim slots immediately? -> No; the stock branch only calls vtable +0x1E8 on the sender. Anim slot 7 is later in mission 0x10 init.` (evidence: `0x0043C788..0x0043C7A6`, `0x0073E013..0x0073E08E`)
- `[RESOLVED] OQ-06 - What fields does Queue_Mission(0x10,0) write? -> If not redundant, MissionClass writes queued mission +0xB4 and clears +0xB8; flag 0 prevents immediate commence.` (evidence: `0x005B35E0..0x005B3642`)
- `[RESOLVED] OQ-07 - What unit function handles mission 0x10 for UnitClass? -> vtable +0x23C resolves to UnitClass::Mission_Deploy_Building @ 0x0073D630.` (evidence: `0x005B3060`, vtable `0x007F5EAC`)
- `[RESOLVED] OQ-08 - Which first unload fields are set after dispatch? -> When path/facing gates pass, +0xF8=0, +0x6D1=1, +0x100..+0x10C timer fields initialized, and +0xBC=3.` (evidence: `0x0073DFBD..0x0073E09D`)
- `[RESOLVED] OQ-09 - What if facing/rate is not ready? -> Mission_Deploy_Building calls locomotor +0x4C(0x4000) when +0x6AF is clear and returns 5, without setting +0x6D1/state 3.` (evidence: `0x0073DF56..0x0073DFBC`)
- `[RESOLVED] OQ-10 - Is UnitClass::Mission_Unload @ 0x00740EF0 the stock refinery 0x15 target? -> No. That function is UnitClass vtable +0x24C / mission 0x14, while this path queues mission 0x10 / vtable +0x23C.` (evidence: `0x005B3060`, vtable entries `0x007F5EAC`, `0x007F5EBC`)
- `[DEFERRED] OQ-11 - Which sender source wins first for every retail replay: 0x16 or PerCellProcess?` (category: requires-different-system-context; reason: owned by sibling timing/source slots; next-step-if-pursued: reconcile parent swarm)
- `[DEFERRED] OQ-12 - Exact draw-frame mapping of +0x6D1 to UnloadingClass sprite on the first rendered frame?` (category: requires-different-system-context; reason: render path not in this radio receiver slice; next-step-if-pursued: verify unit draw path for +0x6D1 and UnloadingClass)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock `BuildingClass::Receive_Radio(0x15)` only queues sender mission `0x10` and returns `1` | `0x0043C788..0x0043C7A6` | Rust `phase_linked` starts visual/pad/pivot side effects directly | `src/sim/miner/miner_dock_sequence.rs::phase_linked` | Split "0x15 received / mission queued" from "mission 0x10 dispatched / unload latch init" | `refinery_radio_0x15_queues_deploy_building_without_immediate_unload`: after modeled 0x15, no cargo drain, no unload timer, no pad snap yet | Do not make radio `0x15` itself the cargo/unload start |
| `Queue_Mission(0x10,0)` writes queued mission and clears init byte but does not commence immediately | `0x005B35E0..0x005B3642` | Rust phase transition may commence same tick | miner dock FSM scheduler / mission timer equivalent | Add a boundary or state representing queued mission before first dispatch-side pivot/latch | `radio_0x15_does_not_dispatch_mission_0x10_until_mission_tick`: same modeled radio step cannot drain or start state 3 | Do not collapse queued mission and dispatched mission into one instantaneous Rust step unless tick-order evidence proves same dispatch context |
| First mission `0x10` dispatch gates on path steps and facing; if facing not ready it calls locomotor turn and returns `5` | `0x0073DEE0..0x0073DFBC` | Rust `phase_linked` starts pivot immediately; `phase_pivoting` resembles this but is entered too early if tied to radio receipt | `phase_linked`, `phase_pivoting` | Treat pivot/facing as dispatch-side mission work; first not-ready pass should not set unload latch/state 3 | `mission_0x10_first_dispatch_turns_without_setting_unload_latch_when_not_east` | Do not seed unload timer before facing gate accepts |
| Dispatch-side unload init writes `+0xF8=0`, `+0x6D1=1`, timer fields, optional slot-7 anim, then state `+0xBC=3` | `0x0073DFBD..0x0073E09D` | Rust maps to display override/sound/link plus `Pivoting -> Unloading`; exact latch/state split is partial | `phase_pivoting`, `phase_unloading`, render/draw state | Start Rust unload visual/cargo gate only after mission-style facing gate; keep an explicit unload-active latch equivalent | `mission_0x10_facing_ready_sets_unload_active_then_state3_before_first_dump_gate` | Do not mark physical pad occupancy or emit DockDeploy solely because `0x15` returned `1` |
| First actual cargo gate is later state 3 against `HarvesterDumpRate * 900.0`, not radio `0x15` | `0x0073E35B..0x0073E374`; INI `HarvesterDumpRate=0.016` | Rust has an `unload_timer` but must start it after dispatch-side latch, not radio receipt | `phase_unloading`, `MinerConfig::unload_tick_interval` | Preserve delay between latch/start visual state and first slot drain | `unload_first_slot_waits_after_mission_0x10_state3_entry` | Do not drain cargo on the radio handoff frame |

### Negative Facts / Do Not Do

- Do not treat `0x15` as a physical pad snap or a move to `GetDockCoord`; the receiver stock branch has no coordinate code.
- Do not write unit/building reciprocal `+0x2E4` links in the stock `DockUnload=yes` `0x15` branch.
- Do not use `UnitClass::Mission_Unload @ 0x00740EF0` as the stock refinery start-unload handler for radio `0x15`; stock `0x15` queues mission `0x10`, which dispatches to `0x0073D630`.
- Do not emit `DockDeploy` sound or set building anim slots in the immediate stock radio receiver; anim slot `7` is later mission `0x10` work.
- Do not begin cargo draining or seed the dump interval before the mission `0x10` facing/path gates pass.

### Concrete Rust Test Names

- `refinery_radio_0x15_queues_deploy_building_without_immediate_unload`
- `radio_0x15_does_not_dispatch_mission_0x10_until_mission_tick`
- `mission_0x10_first_dispatch_turns_without_setting_unload_latch_when_not_east`
- `mission_0x10_facing_ready_sets_unload_active_then_state3_before_first_dump_gate`
- `unload_first_slot_waits_after_mission_0x10_state3_entry`
- `radio_0x15_stock_dockunload_does_not_snap_to_getdockcoord_or_write_pad_link`

### Stale Docs / Follow-up Docs

- Replace wording that says "`0x15` starts unload" with "`0x15` queues sender mission `0x10`; the first dispatch of UnitClass mission `0x10` starts the unload latch/state after path/facing gates pass."
- Replace wording that maps radio `0x15` to `UnitClass::Mission_Unload @ 0x00740EF0` with "`0x15` queues mission `0x10`, UnitClass vtable `+0x23C`, `UnitClass::Mission_Deploy_Building @ 0x0073D630`; `0x00740EF0` is mission `0x14` / vtable `+0x24C`."
- Replace any `phase_linked` doc wording that implies physical pad snap is a `0x15` side effect. The stock receiver branch does not write position or links.

## Sources

- Ghidra read-only decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`.
- Ghidra disassembly: `0x0043C2DD..0x0043C2F1`, `0x0043C6F2..0x0043C7B2`.
- Ghidra read-only decompile/disassembly: `MissionClass::Queue_Mission @ 0x005B35E0`, `0x005B35E0..0x005B3642`.
- Ghidra read-only decompile: `MissionClass::Mission_Dispatch @ 0x005B3060`.
- Ghidra vtable memory: UnitClass vtable `0x007F5C70`, `+0x1E8 -> 0x005B35E0`, `+0x23C -> 0x0073D630`, `+0x24C -> 0x00740EF0`.
- Ghidra read-only decompile/disassembly: `UnitClass::Mission_Deploy_Building @ 0x0073D630`, especially `0x0073DEE0..0x0073E09D`, `0x0073E35B..0x0073E4D0`.
- Ghidra read-only decompile: `UnitClass::Mission_Unload @ 0x00740EF0` as negative comparison.
- Prior reports referenced: `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_REACHABILITY_GHIDRA_REPORT.md`, `UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_TIMING_GHIDRA_REPORT.md`, `UNITCLASS_PERCELLPROCESS_CALLER_TICK_ORDER_GHIDRA_REPORT.md`, `DRIVELOCOMOTOR_ACCEPTED_CELL_ARRIVAL_VISIBILITY_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scanned: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/mod.rs`.
