# ReadyToCommence Aircraft/Building Writer Lifecycles — Ghidra Report

- **Target question:** What are the complete active-YR writer lifecycles, exact byte transitions, ordering, and same-tick readiness consequences for `AircraftClass+0x6D2/+0x6D4` and `BuildingClass+0x6DD`?
- **Non-goals:** Full Aircraft/Building mission handlers, the full mission dispatcher, Unit/Infantry readiness, and MissionClass raw-byte fields.
- **Evidence needed to mark COMPLETE:** An auditable instruction-level writer census with proven receiver identity, each material writer paired with decompile plus assembly/caller evidence, stock/default reachability classifications, concrete lifecycle traces, and Rust-facing acceptance tests.
- **Stop conditions:** Stop at exact lifecycle closure for these three bytes; record rather than expand any dependency that requires a different class, full mission handler, or unrelated authority system.

## Verdict

**Status: COMPLETE for the bounded three-byte lifecycle.**

The older semantic labels are materially wrong:

1. `AircraftClass+0x6D2` is a reusable aircraft action/busy latch used by attack and drop/overfly state machines. It is not merely an “abort flag.”
2. `AircraftClass+0x6D4` is a reusable mission-transition-ready latch. It is initialized to `1`, but generic takeoff, flight, landing, damage, limbo, and destruction do not write it. It is therefore not an `is_landed` byte.
3. `BuildingClass+0x6DD` is a reusable animation/mission-ready latch. Construction completion is only one setter. Runtime animation, guard, attack, service, selling, missile, radio, and successful-commence paths repeatedly set or clear it. It is not a permanent `construction_complete` byte.

The most important ordering result is that `BuildingClass::UpdateAnimation` can set `+0x6DD`, and the same `BuildingClass::Update` invocation can immediately call `ReadyToCommence`, successfully `Commence`, and clear the byte. A Rust implementation that delays this handoff to a later global phase is one native tick late.

## Method and coverage

### Receiver identity

The field accesses below belong to the intended leaf classes, not unrelated objects that happen to have the same displacement:

| Leaf | Primary vtable | RTTI proof | Ready slot |
|---|---:|---|---|
| AircraftClass | `0x007E22A4` | vtable predecessor `0x007E22A0 -> COL 0x007FB4C0 -> TypeDescriptor 0x00817B90`, string `.?AVAircraftClass@@` | `vtable+0x200 @ 0x007E24A4 -> 0x0041B5E0` |
| BuildingClass | `0x007E3EBC` | vtable predecessor `0x007E3EB8 -> COL 0x007FC360 -> TypeDescriptor 0x00818D60`, string `.?AVBuildingClass@@` | `vtable+0x200 @ 0x007E40BC -> 0x00454250` |

The material writer functions also have data xrefs from those vtables. Examples: Aircraft AI `0x00414BB0 <- 0x007E2300`, Approach `0x004155F0 <- 0x007E2510`, Rescue `0x00415960 <- 0x007E2508`, Enter `0x00419C80 <- 0x007E24E4`, Guard `0x0041A5C0 <- 0x007E24C0`, and Commence override `0x0041B870 <- 0x007E2490`; Building Receive_Radio `0x0043C2D0 <- 0x007E4050`, Update `0x0043FB20 <- 0x007E3F18`, OnConstructionComplete `0x00445F80 <- 0x007E4398`, Guard `0x004496B0 <- 0x007E40D8`, Sell `0x00449C30 <- 0x007E4104`, Attack `0x0044ACF0 <- 0x007E40CC`, RepairAndProduce `0x0044B780 <- 0x007E4108`, and Missile `0x0044C980 <- 0x007E410C`.

### Writer-census procedure

Read-only Ghidra `search_byte_patterns` scanned the executable for the little-endian 32-bit displacements `d2 06 00 00`, `d4 06 00 00`, and `dd 06 00 00`. Every hit was mapped to its containing function with `get_function_by_address`; function-scoped `search_instructions` then classified the access width and direction. Decompile established the branch context and disassembly hardened the literal byte value and ordering.

The closed class-relevant access sets are:

- Aircraft `+0x6D2`: **17 direct accesses** — 16 writes in 8 functions plus the Ready read.
- Aircraft `+0x6D4`: **15 direct accesses** — 14 writes in 4 functions plus the Ready read.
- Building `+0x6DD`: **26 direct accesses** — 21 writes in 10 functions plus 5 reads.

All remaining executable hits at those displacements were mapped to other classes. The byte-pattern method also catches ordinary `LEA this+field` address-taking. No aggregate copy other than the already-understood raw save/load substrate reaches these bytes. A compiler-generated pre-biased alias could theoretically hide a field displacement, but no such alias was present in the constructors, leaf vtable methods, lifecycle methods, or raw-state save/load paths inspected here.

## Exact ReadyToCommence predicates

### AircraftClass `0x0041B5E0`

Disassembly `0x0041B5E0..0x0041B60D` gives the exact predicate:

```text
current = dword [this+0xAC]
if current == 0x06 or current == 0x15: return false
if byte [this+0x6D2] != 0 and current != 0x1E: return false
return byte [this+0x6D4] != 0
```

The `0x1E` exception is raw mission-ID behavior. In the verified mission table it is the conditional sibling approach mission whose Aircraft handler is at `0x004155F0`; it is not the standard stock superweapon's initial `0x1A` phase.

### BuildingClass `0x00454250`

Disassembly `0x00454250..0x0045425B` is only:

```text
return byte [this+0x6DD] != 0
```

No mission-ID or type flag is consulted by this leaf predicate.

## AircraftClass `+0x6D2` writer lifecycle

Evidence for every row is the named decompile plus function-scoped instruction search; the exact literal writes are assembly-confirmed at the listed sites.

| Function | Exact write and guard/order | Active in YR |
|---|---|---|
| Constructor `0x00413D20` | `0x00413D6D: [this+0x6D2] = 0`. Initial state. | **Yes**, every aircraft. |
| Aircraft AI `0x00414BB0` | `0x00414BD4: = 0` for every current raw mission except `{0x01,0x1B,0x1E,0x1F}`. The switch preserves, rather than sets, the byte for those four IDs. | **Yes**, every active aircraft tick. |
| Sibling approach handler `0x004155F0` | When distance is `<= 0x300`, queues raw mission `0x1F` first, then `0x00415720: = 1`, then computes/sets the opposite-edge destination. | **Conditional**. Real Aircraft vtable handler; no standard ParaDrop/AmerParaDrop launch caller assigns its `0x1E` entry in the verified stock chain. |
| Standard drop/rescue phase `0x00415960` | `0x0041596C: = 1` at entry. Missing target or payload clears at `0x00415A0A` before target/nav cleanup and queueing exit. Out-of-range clears at `0x004159A5` before queueing the next approach/exit. An in-range payload release returns with the byte still `1`. | **Yes** for the stock `0x1A -> 0x1B` paradrop chain. |
| Enter_Idle_Mode `0x004176F0` | After vtable `+0x1FC` succeeds, vtable `+0x1F8` runs, and current raw mission is `0x19`, it resets mission state `+0xBC=0`, then `0x00417723: = 0`. Other paths do not write it. | **Yes** when this aircraft idle-entry branch is taken. |
| Mission_Attack `0x00417FE0` | State 0 `0x41800C: =0`; state 1 `0x418037: =0`; state 3 `0x4180C6: =0`; state 4 sets `1` at `0x4184DC` or `0x41851D` under the two verified locomotor/action subflags; abort/exit states clear at `0x418BD5`, `0x418BF6`, and `0x418C49`. | **Yes** for armed stock aircraft. |
| Aircraft Commence override `0x0041B870` | If current mission is not raw `0x1E`, `0x0041B879: =0`, then tail-jumps to base `MissionClass::Commence @ 0x005B3570`. If current is `0x1E`, it deliberately preserves the byte. Vtable xref `0x007E2490` is `+0x1EC`, proving this stale local “Override_Mission” label is actually the Commence override. | **Yes** for queued mission promotion; exception conditional on raw `0x1E`. |

### `+0x6D2` semantic and timing conclusions

- Best evidence-backed Rust name: `aircraft_action_latch` or `aircraft_transition_busy`, not `abort` and not a generic airborne flag.
- Setting it normally blocks `ReadyToCommence`; raw current mission `0x1E` is the sole bypass.
- The approach handler queues `0x1F` and then sets the latch. Queue-time Ready sees current still `0x1E`, so the exception permits same-tick Commence. Aircraft Commence also preserves the latch specifically for that old current ID. Once `0x1F` is current, a further promotion is blocked while the latch remains set.
- Standard superweapon Open/Rescue behavior does not need the `0x1E` exception: the `0x1B` phase clears the latch before it queues its next `0x1A`/exit phase.

## AircraftClass `+0x6D4` writer lifecycle

| Function | Exact write and guard/order | Active in YR |
|---|---|---|
| Constructor `0x00413D20` | `0x00413D7B` stores the constructor's established `AL=1`. Initial state is `1`. | **Yes**, every aircraft. |
| Carryall Move `0x00416D50` | State 2 clears at `0x0041710F` only after locomotor secondary `+0x90` returns `1` and target-cell alignment did not restart state 0; state 3 sets at `0x0041717F` on the cargo/`+0x500` branch and at `0x004172CA` after the docking/pickup radio cleanup, before vtable `+0x484(0,1)`. | **Conditional** on `AircraftType+0xDFC Carryall`. Stock `[HIND]` has `Carryall=yes`, `Landable=yes`, `Passengers=10`, but `TechLevel=-1`; scenario/editor spawning can reach it, ordinary build menus cannot. |
| Mission_Enter / Sticky handler `0x00419C80` | States 0–5 converge on `0x00419DCD: =1`. In state 6, assembly proves `0x00419E52` stores the `AL==1` locomotor result, not zero; queue branches also set at `0x419E7D`, `0x419E90`, `0x419E97`. The alternate state-6-to-7 handoff has the only literal clear, `0x00419EF6: =0`. State 7 writes `1` at `0x419FE4` or `0x419FFD` on its next processing paths. | **Yes** for stock aircraft enter/docking transitions. |
| Mission_Guard `0x0041A5C0` | `0x0041A605: =1` immediately before queueing Move on the team/nav path; `0x0041A69F: =1` before the alternate scatter/action call when the outer movement/weapon guard passes. | **Yes** for stock aircraft guard behavior. |

The `0x00419E52` store is an assembly-hardened correction to misleading decompiler output: the immediately preceding locomotor call is tested for `EAX==1`, and the surviving `AL` value written is therefore `1`.

### Verified negatives for `+0x6D4`

There is no writer in Aircraft AI, generic Mission_Attack, generic takeoff/flight processing, the sibling drop handlers, constructor-following Unlimbo `0x00414310`, destructor `0x00414290`, damage/destruction, Reveal, Limbo, or removal. Consequently, this byte cannot be implemented as `is_landed` or recomputed from altitude. It is a narrow transition latch whose sole runtime clear found here is in Carryall Move and the Enter state-6-to-7 handoff.

## Aircraft save/load and concrete trace

Aircraft vtable slots `+0x14/+0x18` resolve to Load `0x0041B430` and Save `0x0041B5C0`. Load calls the FootClass raw-state loader, then reseats vtables/fixes the type and cached dock pointers without writing `+0x6D2/+0x6D4`; Save delegates to the FootClass raw-state saver. Thus both bytes survive save/load as raw authoritative state.

### Stock paradrop depart/return trace

1. Aircraft construction starts `+0x6D2=0`, `+0x6D4=1`; barring excluded current missions, Ready is true.
2. Stock launch queues raw mission `0x1A`. Aircraft AI clears `+0x6D2` because `0x1A` is not in its preserve set. The `0x1A` handler can queue `0x1B`; Ready sees `0/1`, Commence succeeds.
3. The `0x1B` drop phase sets `+0x6D2=1` at entry. While it releases an in-range payload it returns with that busy latch set, and Ready is false because current is not `0x1E`.
4. When it must make another approach or leave, that same handler clears `+0x6D2` before queueing `0x1A` or the exit mission. `+0x6D4` was never changed by generic flight, so Ready becomes true and Commence can promote the queued phase that tick.
5. The exiting aircraft flies off-map; generic AI/destruction performs removal without a `+0x6D4` landing transition. This is the concrete proof that `+0x6D4` does not track ground/air status.

The conditional sibling `0x1E -> 0x1F` trace differs only at the handoff: it queues `0x1F` and sets `+0x6D2=1`; the raw-`0x1E` Ready and Commence exceptions intentionally allow that promotion without clearing the latch.

## BuildingClass `+0x6DD` writer lifecycle

### Complete writers

| Function | Exact write and guard/order | Active in YR |
|---|---|---|
| Constructor `0x0043B740` | `0x0043B944: =0`. Initial state. | **Yes**, every building. |
| Receive_Radio `0x0043C2D0`, case `0x15` | Service flags `UnitRepair +0x16A9`, `UnitReload +0x16AA`, `Hospital +0x16C1`, or `Armory +0x16C2` set `1` at `0x0043C76C`, then queue building mission `0x14` and sender mission `0`. `Bunker +0x16AB` sets `1` at `0x0043C7BD`, then queues building mission `0x14`. `DockUnload +0x16B3` is later and does **not** write the byte. | **Conditional**, but live stock flags exist: GAAIRC/AMRADR UnitReload, GADEPT/NADEPT UnitRepair, base hospital/armory objects, and NATBNK Bunker. Stock ore refineries take the verified no-write DockUnload branch. |
| Update `0x0043FB20` | Calls `UpdateAnimation @ 0x004509D0` first (`0x0043FE22`). First consume calls Ready, additionally requires `+0x534 != 0`, calls Commence, and on true clears at `0x0043FE4D`. After `TechnoClass::AI_Update @ 0x006F9E50`, a second Ready/Commence pair clears at `0x0043FFAD` on true, without the `+0x534` gate. | **Yes**, every active building. |
| OnConstructionComplete `0x00445F80` | On the first-placement/completion side-effect path, `0x004467C9: =1` after placement-related virtual calls and before later power/online continuation. | **Yes**, ordinary construction/deploy completion. |
| Mission_Guard `0x004496B0` | Armed/active path sets `1` at `0x00449701` before operator/target logic and any Attack queue/Commence. The earlier no-weapon/service branch returns without this write. | **Yes** for armed stock buildings. |
| Sell `0x00449C30` | `0x0044AB61: =0` immediately before vtable `+0x280(0x17)` and sell-animation setup; another live sell branch sets sell state `+0xBC=2`, calls GrandOpening(0), then `0x0044A8B5: =0`. State 2 reads the byte and waits while zero. | **Yes** for sell and undeploy/sell-animation paths. |
| Mission_Attack `0x0044ACF0` | With target present, selects weapon, then `0x0044B008: =1` immediately before GetFireError (`vtable+0x3C0`). | **Yes** for armed stock buildings. |
| MissionRepairAndProduce `0x0044B780` | Eight writes: shared Hospital init/rate-threshold clear `0x0044B911`; Armory state-0 clear `0x0044BA15`; UnitRepair state-0 clear `0x0044BB77`; UnitRepair rate threshold clears at `0x0044BD6D` after radio `0x13` succeeds and before radio `0x1C`; completion sets at `0x0044C18E` when `+0x58C==0` before queueing Guard; a completion/reset branch sets at `0x0044C4E7` after vtable `+0x1A0` and state reset; next service cycle clears at `0x0044C577` after clearing anim slots and before timer setup; approach/piggyback branch clears at `0x0044C700` before contact/locomotor work. | **Conditional** on Bunker, ConstructionYard, Hospital, Armory, UnitRepair, or UnitReload type dispatch; all have stock data examples, though some civilian types are scenario-only. |
| Mission_Missile `0x0044C980` | Gated by `BuildingType+0x16BA NukeSilo`. State 0 `0x0044C9C0: =0`, calls GrandOpening(2), then sets state 1. State 1 waits on the byte before GrandOpening(4)/state 2. | **Yes** for stock `[NAMISL] NukeSilo=yes`. |
| UpdateAnimation `0x004509D0` | `0x004511DF: =1` when the active animation reaches its computed terminal frame, or on the special selling raw mission `0x13` frame `0x17`; `0x00451218: =1` when no active animation and either `+0x534==-1` or `+0x10C==0`. | **Yes**, general building animation update. |

### Complete reads

The five non-writer accesses close the census:

| Reader | Use |
|---|---|
| Construction mission slot `+0x244`, `FUN_00449A50` | State 1 waits for nonzero before radio cleanup, GrandOpening, placement continuation, and queueing Guard. |
| Sell `0x00449C30` | State 2 waits for the sell/reverse animation completion latch. |
| Mission_Missile `0x0044C980` | State 1 waits before GrandOpening(4) and advancing state. |
| ReadyToCommence `0x00454250` | Returns byte-nonzero. |
| ChecksumFields `0x004542D0` | Folds the byte with adjacent Building value fields; it is lockstep/checksum-visible. |

### Building semantic and same-tick conclusions

- Best evidence-backed Rust name: `building_anim_mission_ready_latch`.
- Construction completion arms it, but neither makes it permanent nor owns its sole setter.
- Animation completion/no-animation arms it; successful mission Commence consumes it by clearing.
- Sell, service, and missile handlers clear it immediately before starting a waitable animation/action, and their completion paths set it again.
- `UpdateAnimation -> Ready -> optional +0x534 gate -> Commence -> clear -> Techno AI -> Ready -> Commence -> clear` is one object-local ordered bracket. The set and successful consume can occur in one `BuildingClass::Update` call.

## Building save/load, lifecycle negatives, and concrete trace

Building vtable slots `+0x14/+0x18` resolve to Load `0x00453E20` and Save `0x00454190`. Load uses the Techno raw-state loader, then calls the lightweight Building constructor `0x0043B680`; that re-establishes vtables/dynamic members but does not write `+0x6DD`. Save delegates to Techno raw-state save and serializes only additional dynamic vectors. Therefore the byte survives save/load, and `ChecksumFields @ 0x004542D0` proves it affects synchronized state.

Global displacement closure and direct inspection found no `+0x6DD` writer in Building destructor `0x0043BCF0`, Unlimbo `0x00440580`, Limbo `0x00445880`, generic Reveal/Conceal, damage/destruction effects, ordinary building self-repair toggling, or final removal. Those paths preserve the byte until raw object disposal; only the writer functions enumerated above mutate it.

### Construction then sell/undeploy trace

1. Constructor initializes `+0x6DD=0`.
2. The first placement/completion path sets it to `1` at `0x004467C9` after placement side effects.
3. On the next active object update, `UpdateAnimation` runs before Ready. If a queued mission exists and Commence succeeds, the first eligible consume clears the byte in that same update; otherwise it stays armed. Later terminal/no-animation conditions can set it again.
4. Sell/undeploy begins by clearing the byte before the reverse/opening animation and entering the wait state.
5. `UpdateAnimation` detects the selling terminal frame `0x17`, the computed terminal frame, or the no-animation condition and sets the byte to `1`.
6. Sell state 2 observes nonzero and performs the remaining conversion/refund/removal path. This reuse after construction disproves the permanent `construction_complete` interpretation.

## Stock/default reachability summary

| Mechanism | Classification | Evidence |
|---|---|---|
| Aircraft constructor, AI, attack, guard, enter, Commence | **Yes** | Leaf vtable bindings plus stock Aircraft types. |
| Standard superweapon `0x1A/0x1B` drop chain | **Yes** | Verified stock SuperClass launch/spawner callsites and PDPLANE data. |
| Sibling raw `0x1E/0x1F` handler pair | **Conditional** | Leaf vtable handlers exist; no standard ParaDrop/AmerParaDrop launch caller assigns `0x1E`. |
| Carryall Move | **Conditional** | `AircraftType+0xDFC`; stock HIND has `Carryall=yes` but `TechLevel=-1`. |
| Building construction/update/animation/guard/attack/sell | **Yes** | Normal instantiated BuildingClass paths. |
| Service/repair/produce writers | **Conditional**, stock-backed | Stock service depots, airfields, bunker, construction yards, and scenario civilian service types set relevant flags. |
| Missile writer | **Yes** | Stock NAMISL has `NukeSilo=yes`. |
| Stock refinery DockUnload setting `+0x6DD` | **No** | DockUnload radio branch queues only the sender's `0x10`; it has no `+0x6DD` write. |

## Current Rust delta and implementation handoff

No Rust was changed in this investigation. Current source remains structurally incomplete for these bytes:

- `src/sim/mission/verb.rs:164..181` gives `ReadySnapshot` only `{category,is_driving}` and returns `true` for Building and Aircraft.
- `src/sim/game_entity.rs:314..340,399,525` has building-up/down, generic animation, aircraft mission, and `MissionCom`, but no authoritative equivalents of these three latches.
- `src/sim/world/world_hash.rs:36..58,816` hashes `MissionCom`; the new latch state would also need explicit stable hash folding and save-schema handling.
- `src/sim/world/mod.rs:1820..1879,2004..2006` advances build-up/down as late global phases, not as the verified Building object-local `UpdateAnimation -> Ready/Commence` bracket.
- `src/sim/production/production_sell.rs:705..774` sells/refunds/uninits immediately, so it cannot yet express the verified clear -> animation completion -> read/continue lifecycle.

### Bounded implementation units

1. **Authoritative latch state and predicate.** Add three plainly named bytes/bools to the appropriate Rust capability state, serialize and hash them, initialize Aircraft `(false,true)` and Building `false`, and implement the exact raw-current-mission Aircraft predicate plus Building latch predicate. Do not derive aircraft readiness from altitude.
2. **Aircraft writer hooks.** Route the enumerated AI, attack, enter, guard, standard drop, conditional sibling approach, Carryall, and Commence writes through one aircraft-lifecycle API. Preserve write-before/after-queue ordering and the raw-`0x1E` exception.
3. **Building object-local bracket.** Route construction, animation, radio, guard/attack, service, missile, sell/undeploy writes through one building-lifecycle API and execute `UpdateAnimation -> Ready -> Commence -> clear` at the verified per-object position, including the second post-Techno-AI consume point.

These are behavior components on the flat `GameEntity`; no C++ class tree or Rust `dyn` hierarchy is warranted.

## Required acceptance tests

1. `aircraft_ready_truth_table_exact`: excluded current IDs `0x06/0x15`; action latch block; raw `0x1E` exception; transition latch final gate.
2. `aircraft_ai_preserves_action_only_for_01_1b_1e_1f`: all other current IDs clear it.
3. `stock_paradrop_rescue_clears_busy_before_queueing_next_phase`: set during payload work, clear before queue/Commence on approach/exit.
4. `sibling_1e_queue_set_and_commence_same_tick`: queue `0x1F`, set busy, Ready still true because old current is `0x1E`, Commence preserves busy.
5. `aircraft_transition_ready_is_not_altitude`: ordinary takeoff/flight/landing does not invent writes; Enter's state-6 clear and next-state set follow exact order.
6. `building_construction_arms_then_successful_commence_consumes`: constructor 0, completion 1, UpdateAnimation before Ready, successful Commence clears in the same object update.
7. `building_no_queue_does_not_consume_ready_latch`: Ready true but Commence false leaves the latch set.
8. `building_sell_waits_for_animation_latch`: sell clears, terminal/no-animation update sets, state-2 continuation reads it before removal/refund.
9. `building_update_has_two_ordered_consume_points`: first requires `+0x534!=0`; second does not and occurs after Techno AI.
10. `ready_latches_roundtrip_and_hash`: mid-action Aircraft and mid-sell Building save/load preserve all bytes and produce the same state hash/checksum.
11. `stock_refinery_radio_0x15_does_not_set_building_ready_latch`: DockUnload stays separate from service mission `0x14`.
12. `limbo_unlimbo_do_not_synthesize_latch_writes`: lifecycle transitions preserve the serialized bytes unless an enumerated mission/animation writer runs.

## Corrections required in older research prose

Use these exact replacement statements when the owning documents are next audited; this report intentionally does not edit them:

> `AircraftClass+0x6D2` is a reusable action/busy latch written by Aircraft AI, drop/approach, Attack, idle-entry, and Commence paths. Nonzero blocks ReadyToCommence except while raw current mission is `0x1E`; it is not merely an abort flag.

> `AircraftClass+0x6D4` is a mission-transition-ready latch initialized to `1` and written by Carryall Move, Mission_Enter/Sticky, and Mission_Guard. No generic takeoff, flight, landing, damage, limbo, or destruction writer was found, so it must not be labeled or implemented as an `is_landed` flag.

> `BuildingClass+0x6DD` is a reusable animation/mission-ready latch, not a permanent construction-complete flag. Construction completion is one setter; UpdateAnimation, Guard, Attack, radio/service, repair/produce, Sell, Missile, and successful Commence repeatedly set or clear it, and the byte is save/checksum-visible.

Specifically, `READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md` must replace “landed-ready flag” and “construction-complete/permanently open” language; `BUILDINGCLASS_SAVE_LOAD_GHIDRA_REPORT.md` should rename its `+0x6DD` value-field row from “ConstructionComplete flag” to “animation/mission-ready latch.”

## Residual uncertainty and negative facts

- The semantic Rust field names above are evidence-based descriptions, not recovered source identifiers; original C++ member names remain unknown.
- The raw `0x1E/0x1F` sibling handlers are binary-live through the Aircraft vtable but are not on the verified standard stock superweapon launch chain. Treat their normal-game frequency as conditional.
- This investigation does not claim the full semantics of every service, attack, sell, or missile state machine. It closes only the exact three-byte reads/writes, their local guards/order, and their readiness consequences.
- No RNG is consumed by either Ready predicate or by the literal byte writes themselves. Surrounding handlers may consume RNG outside this bounded field lifecycle.
- No direct writer exists in the scoped reveal/conceal, limbo/unlimbo, damage/destruction, or final removal paths.

## Sources

### Fresh read-only Ghidra evidence

- `search_byte_patterns`: `d2 06 00 00`, `d4 06 00 00`, `dd 06 00 00`; every hit classified with `get_function_by_address` and function-scoped `search_instructions`.
- Decompile/disassembly: Aircraft `0x00413D20`, `0x00414BB0`, `0x004155F0`, `0x00415960`, `0x00416D50`, `0x004176F0`, `0x00417FE0`, `0x00419C80`, `0x0041A5C0`, `0x0041B430`, `0x0041B5C0`, `0x0041B5E0`, `0x0041B870`.
- Decompile/disassembly: Building `0x0043B680`, `0x0043B740`, `0x0043C2D0`, `0x0043FB20`, `0x00445F80`, `0x004496B0`, `0x00449A50`, `0x00449C30`, `0x0044ACF0`, `0x0044B780`, `0x0044C980`, `0x004509D0`, `0x00453E20`, `0x00454190`, `0x00454250`, `0x004542D0`.
- RTTI/vtable memory and xrefs: Aircraft `0x007E22A0/0x007E22A4`, Building `0x007E3EB8/0x007E3EBC`, and the writer slot xrefs listed under Receiver identity.

### Existing documents reconciled

- `docs/research/READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md`
- `docs/research/READYTOCOMMENCE_S5_BLOCKER_CLOSURE_AND_FEAR_SEQUENCE_GATE_GHIDRA_REPORT.md`
- `docs/research/AIRCRAFTCLASS_GHIDRA_REPORT.md`
- `docs/research/PARADROP_MISSION_TRANSITIONS_GHIDRA_REPORT.md`
- `docs/research/PARADROP_DROP_CADENCE_GHIDRA_REPORT.md`
- `docs/research/MISSIONCLASS_STATE_MACHINE.md`
- `docs/research/BUILDINGCLASS_SAVE_LOAD_GHIDRA_REPORT.md`
- `docs/research/BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md`
- `docs/research/MISSION_REPAIR_AND_PRODUCE_GHIDRA_REPORT.md`
- `docs/research/BUILDING_MISSIONREPAIRANDPRODUCE_DOCKUNLOAD_REACHABILITY_GHIDRA_REPORT.md`

### Stock data and Rust surfaces

- `ini/rulesmd.ini`: Aircraft defaults comment near `Carryall/Landable`; `[HIND]`; `[GAAIRC]`, `[AMRADR]`, `[GADEPT]`, `[NADEPT]`, `[NAMISL]`, `[NATBNK]`; `ini/rules.ini` fallback hospital/armory definitions.
- `src/sim/mission/verb.rs`
- `src/sim/game_entity.rs`
- `src/sim/world/world_hash.rs`
- `src/sim/world/mod.rs`
- `src/sim/production/production_sell.rs`

