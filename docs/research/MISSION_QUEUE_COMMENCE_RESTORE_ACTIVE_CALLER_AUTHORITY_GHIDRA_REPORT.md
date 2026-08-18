# Mission Queue / Commence / Restore Active Caller Authority — Ghidra Research Report

**Date:** 2026-07-22  
**Address(es):** `MissionClass::Queue_Mission @ 0x005B35E0`, `MissionClass::Commence @ 0x005B3570`, `MissionClass::Restore_Mission @ 0x005B36B0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** executable-wide static census of direct calls and literal vtable calls through `+0x1E8`, `+0x1EC`, and `+0x1F8`; receiver proof; Queue mission/`commence_now` arguments; immediate Ready/Commence/Restore order; immediate Target/NavCom context; current Rust mapping.  
**Non-Scope:** settled base verb bodies, B8/CC census, leaf Ready predicates, full mission-handler semantics, full dispatcher, Assign/Override caller census, Aircraft override internals beyond binding/call-through, and native-save compatibility.  
**Confidence:** HIGH for the claimed static slice  
**Active in YR:** Yes; individual content-, mission-, editor-, script-, team-, and superweapon-gated contexts are marked Conditional.

## 0. Pre-flight Contract

### Target question

What is the complete active-YR, receiver-proven direct and virtual caller surface for Queue (`0x005B35E0/+0x1E8`), Commence (`0x005B3570/+0x1EC`), and Restore (`0x005B36B0/+0x1F8`), including each Queue `commence_now` value and synchronous same-tick order?

### Non-goals

- Re-proving the settled mutation bodies.
- Re-proving B8/CC or leaf Ready predicates.
- Explaining handlers beyond the immediate call context.
- Mapping Assign/Override; that is a sibling investigation.
- Writing Rust.

### Evidence needed to mark COMPLETE

1. Program-wide instruction search for every `CALL` operand containing each literal slot displacement, with untruncated counts.
2. Direct code/data xrefs to all three base bodies.
3. Decompile plus assembly context for every hit; receiver flow must establish Mission/Radio/Techno ancestry or reject the hit.
4. COL -> TypeDescriptor -> slot-byte proof for the common class vtables and for every rejected same-offset receiver class.
5. Current Rust read-only scan for the three verb functions, production callers, and direct mission writers.

### Stop conditions

Stop when every static direct call and every literal `CALL [reg+0x1E8/1EC/1F8]` is classified, false receivers are named, all open questions are resolved, and one zero-add pass finds no new callsite.

## 1. Executive Result

The census is complete for the stated static form in the current `gamemd.exe` analysis:

| Search | Raw literal-slot hits | Receiver-proven mission calls | Rejected same-offset calls | Direct code callers of base body |
|---|---:|---:|---:|---:|
| `CALL [...+0x1E8]` | 259 | **256 Queue calls** | 3 | 1 (`Aircraft Queue override -> base`) |
| `CALL [...+0x1EC]` | 60 | **59 Commence calls** | 1 | 1 (`Aircraft Commence override -> base`) |
| `CALL [...+0x1F8]` | 3 | **3 Restore calls** | 0 | 1 (`Techno restore wrapper -> base`; Foot wrapper chains through it) |

Evidence: `search_instructions(mnemonic=CALL, operand_pattern=0x1e8/0x1ec/0x1f8, limit=1000)` scanned 1,151,419 instructions for Queue and returned `truncated=false`; the two smaller searches were also untruncated. Direct xrefs were read with `get_function_xrefs` at the three body addresses. Active in YR: Yes.

Load-bearing consequences:

1. A `commence_now` value whose **low byte is nonzero** is not an advisory tag. Base Queue receives the parameter as `char` and synchronously performs `Ready(+0x200) -> Commence(+0x1EC)` before returning. Aircraft forwards a full dword, but the base reads only that low byte. Active in YR: Yes (`0x005B35E0` decompile; `0x005B3621..0x005B3641`; Aircraft wrapper `0x0041BA90`).
2. A `commence_now` value whose **low byte is zero** never promotes inside Queue, even when a higher byte is nonzero. Callers either leave the mission queued, explicitly call Commence, or explicitly run Ready then Commence. Active in YR: Yes; the complete crosswalk is below.
3. Direct Commence is a first-class authority path: there are receiver-proven calls from object update, lifecycle, mission handler, scenario load, event/team, and locomotor contexts outside Queue. Active in YR: Yes/Conditional by owner.
4. Restore is not a bare mission pop on concrete Technos. Derived `+0x1F8` wrappers restore saved Target or NavCom only when the base pop succeeds. Active in YR: Yes (`0x007013E0`, `0x004D8F80`).
5. Numeric slot search without receiver proof is wrong: Anim, Bullet, and Particle each have unrelated `+0x1E8`; Anim also has unrelated `+0x1EC`. Active in YR as mission callers: No.

## 2. Census Method and Completeness Boundary

### 2.1 Search method

1. Direct xrefs to `0x005B35E0`, `0x005B3570`, and `0x005B36B0` enumerated direct calls and vtable data references.
2. Whole-program instruction search enumerated every decoded indirect `CALL` whose operand contains `0x1E8`, `0x1EC`, or `0x1F8`.
3. Every hit was grouped by containing function. Seven Queue and four Commence hits lack a Ghidra function boundary; their bytes and surrounding instructions were inspected read-only with `get_assembly_context`. No function was created.
4. For each hit, the receiver was followed from ECX and its source: typed `this`, a concrete class array, a type allocator result, a linked locomotor owner, a radio sender, a spawned/slave/team member, or a building/aircraft/foot field.
5. A final repeat of all three instruction searches produced the same counts and no new hit.

### 2.2 Completeness limit

This is executable-wide and complete for direct calls plus decoded literal-displacement virtual calls in the loaded retail YR executable. It does not claim to discover a hypothetical runtime-generated function pointer call that never materializes as a direct target or a literal vtable displacement. No such extra path was found in direct xrefs, vtable data, or the zero-add pass. This limit does not downgrade the bounded result.

## 3. Vtable Identity and Binding Proof

Each row was independently walked from `vtable-4` to COL, from `COL+0x0C` to TypeDescriptor, then read at slots `+0x1E8..+0x200`.

| Class / vtable | TypeDescriptor proof | `+1E8` | `+1EC` | `+1F0` | `+1F4` | `+1F8` | `+200` |
|---|---|---|---|---|---|---|---|
| Aircraft `0x007E22A4` | COL `0x007FB4C0` -> TD `0x00817B90` `.?AVAircraftClass@@` | `0x0041BA90` override | `0x0041B870` override | `0x0041B9F0` | `0x0041BB30` | `0x004D8F80` | `0x0041B5E0` |
| Building `0x007E3EBC` | COL `0x007FC360` -> TD `0x00818D60` `.?AVBuildingClass@@` | `0x005B35E0` | `0x005B3570` | `0x005B2FD0` | `0x007013A0` | `0x007013E0` | `0x00454250` |
| Foot `0x007E8C94` | COL `0x00800948` -> TD `0x00817B78` `.?AVFootClass@@` | base | base | base | `0x004D8F40` | `0x004D8F80` | base Ready |
| Infantry `0x007EB058` | COL `0x008033B8` -> TD `0x00825508` `.?AVInfantryClass@@` | base | base | base | `0x004D8F40` | `0x004D8F80` | `0x00521B60` |
| Mission `0x007EDCC0` | COL `0x00805D28` -> TD `0x00817B18` `.?AVMissionClass@@` | base | base | base | base | base | base Ready |
| Radio `0x007F0508` | COL `0x00808590` -> TD `0x00817B38` `.?AVRadioClass@@` | base | base | base | base | base | base Ready |
| Techno `0x007F4960` | COL `0x0080C058` -> TD `0x00817B58` `.?AVTechnoClass@@` | base | base | base | `0x007013A0` | `0x007013E0` | base Ready |
| Unit `0x007F5C70` | COL `0x0080CC68` -> TD `0x00842D80` `.?AVUnitClass@@` | base | base | base | `0x004D8F40` | `0x004D8F80` | `0x00744270` |

`base` means Queue `0x005B35E0`, Commence `0x005B3570`, Assign `0x005B2FD0`, Override `0x005B3650`, Restore `0x005B36B0`, or Ready `0x004E0140` as appropriate. Active in YR: Yes.

### 3.1 Rejected receiver slots

| Hit | Receiver proof | Actual slot target | Verdict |
|---|---|---|---|
| `AnimClass::AI 0x00423C36`, `+1E8` | Anim vtable `0x007E3354`; COL `0x007FBA60` -> TD `0x008182C8` `.?AVAnimClass@@` | `0x00423930`, bounce result | Not Queue; Active in YR as mission caller: No |
| `BulletClass` draw `0x0046826F`, `+1E8` | Bullet vtable `0x007E46E4`; COL `0x007FC7B0` -> TD `0x0081AF70` `.?AVBulletClass@@` | `0x00468000`, bullet draw/mechanism method | Not Queue; Active in YR as mission caller: No |
| `ParticleClass::Draw_It 0x0062CFB5`, `+1E8` | Particle vtable `0x007EF954`; COL `0x00807BE8` -> TD `0x008366E8` `.?AVParticleClass@@` | `0x0062D830`, particle method | Not Queue; Active in YR as mission caller: No |
| `AnimClass::DrawIt 0x00422D14`, `+1EC` | Same Anim COL proof | `0x00425510`, animation frame/count method | Not Commence; Active in YR as mission caller: No |

`AnimClass::AI 0x00424B04` is **not** rejected: that call's receiver is a newly allocated Infantry (`piVar16` from the InfantryType allocator), so it is a real `Queue(0x0F,0)` call even though the containing function is Anim AI.

## 4. Immediate Verb and Wrapper Order

### 4.1 Queue order used by every caller

The settled body was not re-investigated. Cold decompile plus assembly were used only to anchor caller interpretation:

```text
Queue(m, commence_now)
  -> apply guards/redundancy and possibly write queued/B8
  -> if (commence_now & 0xff) != 0:
       call receiver Ready +0x200
       if nonzero: call receiver Commence +0x1EC
  -> return synchronously
```

The Ready call is after the attempted queue write. A false Ready leaves the queued mission present. A `m=-1` Queue call does not write or clear the queue; with a zero low byte in `commence_now` it is a complete no-op. Evidence: the `char param_3` signature in the `0x005B35E0` decompile, `0x005B35FC..0x005B3641`, and the full-dword Aircraft forwarder at `0x0041BA90`. Active in YR: Yes.

### 4.2 Concrete Restore order

| Surface | Exact same-call-stack order | Evidence | Active in YR |
|---|---|---|---|
| Techno/Building `+1F8` | base Restore; if success, `Set_Target(saved +0x2B8)`; return true; otherwise no target write | `0x007013E0`; vtables `0x007E3EBC`, `0x007F4960` | Yes |
| Foot/Infantry/Unit/Aircraft `+1F8` | call Techno wrapper above; if success, `Set_Destination(saved +0x5A8,1)`; return true; otherwise no NavCom write | `0x004D8F80`; derived vtables above | Yes |
| Mission/Radio `+1F8` | base Restore only | `0x005B36B0`; vtables `0x007EDCC0`, `0x007F0508` | Yes as substrate; concrete gameplay normally uses derived wrappers |

The wrapper fields correspond to the companion override wrappers: Techno saves Target `+0x2B4 -> +0x2B8`; Foot saves NavCom `+0x5A4 -> +0x5A8`. Evidence: `0x007013A0`, `0x004D8F40`. Active in YR: Yes.

## 5. Complete Restore Caller Ledger

| Callsite / owner | Receiver and gate | Exact local order | Active in YR |
|---|---|---|---|
| `0x00417706`, Aircraft `Enter_Idle_Mode` | `this`; first calls `+1FC` suspended-present predicate | `HasSuspended -> Restore`; if restored current is `0x19`, clear mission substate and Aircraft `+0x6D2` | Yes, normal aircraft idle/arrival path |
| `0x00707A4B`, `TechnoClass::PointerExpired` | `this`; expired pointer matched current Target and sensor/owner exception did not preserve it | clear Target -> `HasSuspended -> Restore`; then Aircraft mission-`0x19` cleanup | Yes, conditional on target expiry |
| `0x0070D50D`, `TechnoClass::StopAllTargeting` | each `g_TechnoClass_Array` member whose Target equals the stopped object, except the explicit capture exception | `Restore` unconditionally -> Aircraft mission-`0x19` cleanup -> if Target still equals stopped object, clear Target | Yes, conditional on global target invalidation |
| direct `0x007013E3`, Techno restore wrapper | receiver forwarded in ECX | base Restore -> conditional saved Target restore | Yes |
| direct `0x004D8F83`, Foot restore wrapper | receiver forwarded in ECX | Techno wrapper above -> conditional saved NavCom restore | Yes |

There are no other decoded direct or `+0x1F8` Restore callers. All three literal slot hits are true Mission-family receivers; no same-offset false receiver survived.

## 6. Complete Commence Caller Ledger

Relation codes: `R>C` = Ready then conditional Commence; `Q0>C` = Queue with flag 0 then unconditional Commence; `Q0>R>C` = Queue flag 0 then explicit Ready/conditional Commence; `Q1` = Queue flag 1 owns internal `R>C`; `C` = direct Commence without an immediately adjacent Queue/Ready. All occur synchronously in the named owner.

| Commence callsite(s) | Owner / local relation | Scheduling context | Active in YR |
|---|---|---|---|
| `0x00415058` | Aircraft AI, `R>C` | per-object Aircraft update | Yes |
| `0x00417B63`, `0x00417B9B` | Aircraft `Enter_Idle_Mode`: `Q0>R>C`; separate queued-approach `C` | arrival/idle callback | Yes |
| `0x00419262` | Aircraft radio path, `Q0`, adjacent destination/radio work, then `C` | `Receive_Radio` | Conditional on radio command |
| `0x0041ABA8` | Aircraft `Set_Destination`, `Q0>R>C` | NavCom writer | Yes |
| `0x0041B3F8` | aircraft creation/load helper, `Q0>C` | object initialization | Conditional on aircraft creation/load branch |
| `0x0043FE43`, `0x0043FFA3` | Building `Update`, `R>C` | same Building object update; no global deferral | Yes |
| `0x00443C54` | Building gate toggle, `Q0>C` | command/mission transition | Conditional on gate toggle |
| `0x0044533F` | Building exit-object branch, `C` | production exit handler | Conditional on produced object branch |
| `0x00446EB1` | construction completion, `Q0>C` | same completion call | Conditional on construction finishing |
| `0x0044718F` | power/gate command, `Q0>C` | command handler | Conditional on command |
| `0x0044979C` | Building helper, `Q0>C` | mission handler helper | Conditional |
| `0x0044AF73`, `0x0044AFDA`, `0x0044B148` | Building Attack exits, `Q0>C` | Building mission dispatch | Yes while Attack handler reaches exit branches |
| `0x0044DD75` | Building service/dock helper, `Q0>C` | radio/service helper | Conditional |
| `0x004525D6` | Building animation/tracker helper, `Q0>C` | Building update helper | Conditional |
| `0x004B5E76` | DropPod locomotor fragment, `C` | linked Foot locomotor processing | Conditional on DropPod locomotion |
| `0x004C7696` | Event execute, `Q0>C` | synchronized event command | Conditional on event type |
| `0x004CCBAD` | Fly locomotor owner, `R>C` | linked Aircraft locomotor process | Yes for Fly locomotion branch |
| `0x004D5461`, `0x004D54B3` | Foot Hunt, `Q0>R>C` twice | Foot mission dispatch | Yes for Hunt branches |
| `0x004D6D45` | Foot AreaGuard, `Q0>C` | Foot mission dispatch | Yes for branch |
| `0x004D9082`, `0x004D91DB` | Foot radio, `Q0>R>C` | `Receive_Radio` | Conditional on radio message/current mission |
| `0x004D9466` | Foot Enter, `C` | Foot mission dispatch | Yes for Enter branch |
| `0x004DDFD0` | Foot Rescue, `Q0>C` | Foot mission dispatch | Conditional on AI Rescue |
| `0x0051812E` | boundaryless Infantry init fragment, `Queue(-1,0) -> Queue(5,0) -> C` | Infantry lifecycle/init | Yes for branch; first Queue is a no-op |
| `0x0051A8C5` | Infantry `PerCellProcess`, `C` | same-object per-cell arrival | Yes |
| `0x0051BC51`, `0x0051BF03` | Infantry AI, `C` | per-object Infantry update | Yes |
| `0x0051FEB0` | Infantry helper, `Q0>C` | mission/action helper | Conditional |
| `0x0054B4B1` | Jumpjet completion special sequence | `Queue(5,1)` internal `R>C` -> write B8=1 -> second Ready -> conditional second Commence | Yes for Jumpjet arrival |
| `0x005B363B` | base Queue internal `R>C` | inside any `commence_now!=0` Queue call | Yes |
| `0x0065DCF9` | object-placement helper, earlier `Q0`, then `C` | creation/placement helper | Conditional |
| `0x0065E46B` | placement helper, `Q0>C` | object creation | Conditional |
| `0x0065E809`, `0x0065EA62`, `0x0065EBE9` | three type-creation helpers, dynamic mission `Q0>C` | scenario/object creation | Conditional |
| `0x0065F2A9` | Chronosphere warp list, `C` | superweapon warp application | Conditional on Chronosphere |
| `0x006CDDB7` | Super launch, `Q0>C` | superweapon launch | Conditional |
| `0x006EBE3C` | Team convoy without target, `Q0>R>C` | Team AI | Conditional on team script |
| `0x006EF3D6` | Team helper, `Q0>R>C` | Team AI | Conditional on team script |
| `0x006F6E49` | `TechnoClass::Unlimbo`, `R>C` | synchronous lifecycle call | Yes |
| `0x00718C9F` | Teleport locomotor, `R>C` | linked Foot locomotor process | Conditional on teleport locomotion |
| `0x00736473`, `0x007366FD` | Unit AI: `Q0>R>C`; separate `R>C` | per-object Unit update | Yes |
| `0x00737AF6` | Unit radio, `Q0>R>C` | `Receive_Radio` | Conditional on radio message |
| `0x0073AA70`, `0x0073ACD1` | Unit `PerCellProcess`: `Q0>C`; separate `R>C` | same-object per-cell arrival | Yes |
| `0x0073D6DB` | Unit DeployBuilding, `C` | mission handler | Conditional on branch |
| `0x0073DE34`, `0x0073DECB` | Unit DeployBuilding, two `Q0>C` branches | mission handler | Conditional on branch |
| `0x0073DF43`, `0x0073E12F` | Unit DeployBuilding, `R>C` | mission handler | Conditional on branch |
| `0x0073E174` | Unit DeployBuilding, `C` | mission handler | Conditional on branch |
| `0x0073E283` | Unit DeployBuilding state 4, `Queue(10,0)`, intervening contact/radio work, explicit `C` | same mission dispatch; after unload cleanup | Yes for miner exit branch |
| `0x0074363B` | scenario Unit load, dynamic `Q0>R>C` | scenario load, before next object | Yes when loading Units section |
| direct `0x0041B880` inside Aircraft `+1EC` override | if old current is not `0x1E`, clear Aircraft `+0x6D2`; tail-call base Commence | any Aircraft Commence | Yes |

`0x00422D14` is the sole rejected `+0x1EC` hit and is not included in the 59 mission calls. There are no other direct base-Commence code callers.

## 7. Complete Queue Caller Ledger

Notation is `callsite=mission/commence_now`. `dynamic/0` means the mission comes from a live caller value rather than an immediate. Every `/1` call performs Queue's internal `Ready -> conditional Commence` before returning; every `/0` call only attempts the queued-field write. Explicit later Commence calls are listed in Section 6. Receiver classification was checked at every hit; this table contains all 256 Mission-family calls and none of the three rejected same-offset collisions.

| Queue callsite(s) and arguments | Receiver / owner | Scheduling context and activity |
|---|---|---|
| `0x00414B9E=1/0` | Aircraft `this` in boundary fragment preceding AI | Active Aircraft update branch |
| `0x00415625=4/0`, `0x00415714=31/0` | Aircraft `this`, paradrop path | Conditional Aircraft paradrop |
| `0x00415901=4/0`, `0x0041594A=27/0` | Aircraft `this`, open/release path | Conditional Aircraft action |
| `0x004159B8=26/0`, `0x00415A33=4/0` | Aircraft `this`, rescue path | Conditional Aircraft Rescue |
| `0x00415C05=2/0` | Aircraft parameter receiver, enter eligibility path | Conditional command target |
| `0x00417B4B=dynamic/0` | Aircraft `this`, `Enter_Idle_Mode` | Active arrival/idle transition; explicit Ready/Commence follows |
| `0x00418CB4=4/0` | Aircraft `this`, Attack | Active mission branch |
| `0x004191B7=dynamic(2-or-7)/0`, `0x0041924B=2/0`, `0x00419385=4/0` | Aircraft `this`, `Receive_Radio`; one sender Aircraft receiver | Conditional radio traffic |
| `0x00419CF9=2/1`, `0x0041A0E3=2/0`, `0x0041A112=2/0`, `0x0041A12D=5/0` | Aircraft `this`, Enter mission | Active mission-state branches; first is synchronous Queue-owned promotion |
| `0x0041A60C=2/0`, `0x0041A67C=2/0`, `0x0041A7F8=7/0`, `0x0041A834=1/0`, `0x0041A8D6=1/0` | Aircraft `this`, Guard | Active Guard branches |
| `0x0041A9C5=1/0` | Aircraft `this`, adjacent Guard helper | Active conditional branch |
| `0x0041AB90=dynamic/0` | Aircraft `this`, `Set_Destination` | NavCom writer; explicit Ready/Commence follows |
| `0x0041B3EE=dynamic/0` | newly created/loaded Aircraft | Object creation/load; explicit Commence follows |
| `0x0041DB1C=1/0`, `0x0041DB9C=4/0` | Aircraft `this`, late class helpers | Conditional Aircraft behavior |
| `0x00424B04=15/0` | newly allocated Infantry, not Anim `this` | Active Anim-AI creation branch; receiver proven by allocation/use flow |
| `0x0043C773=20/0`, `0x0043C7A0=16/0`, `0x0043C7C4=20/0`, `0x0043C7D4=0/0`, `0x0043CC41=20/0`, `0x0043CC67=5/0` | Building `this` or radio sender Techno, `Receive_Radio` | Conditional active radio traffic |
| `0x00442FBD=15/0`, `0x0044323F=1/0`, `0x00443268=2/0`, `0x00443277=15/0` | spawned survivor Foot/Infantry receivers | Active destruction/survivor creation branches |
| `0x00443C4A=19/0` | Building `this`, gate toggle | Conditional command; explicit Commence follows |
| `0x0044408B=2/0`, `0x00444187=16/0`, `0x00444439=2/0`, `0x004445F0=16/0`, `0x004448D6=2/0`, `0x0044490F=11/0`, `0x00444CEF=2/0`, `0x00444D3C=11/0`, `0x00444ED8=10/0` | produced/exiting Techno receivers in Building exit-object logic | Active production exit branches |
| `0x00446EA7=10/0`, `0x00446F8F=5/0` | Building `this`, construction completion | Active completion branches; first has explicit Commence |
| `0x00447185=19/0` | Building `this`, power/gate command | Conditional command; explicit Commence follows |
| `0x00449792=1/0`, `0x0044994A=20/0`, `0x004499B5=20/0` | Building `this`, mission helper | Active conditional branches |
| `0x00449AE2=5/0` | Building `this`, mission helper | Active conditional branch |
| `0x0044A0AE=2/0`, `0x0044A565=15/0`, `0x0044A783=2/0`, `0x0044AB44=5/0` | Building `this` and released Techno receivers, Sell | Active sell lifecycle branches |
| `0x0044AF69=5/0`, `0x0044AFD0=5/0`, `0x0044B13E=5/0` | Building `this`, Attack | Active mission branches; explicit Commence follows each |
| `0x0044B7E6=5/0`, `0x0044B9CD=5/0`, `0x0044BB62=5/0`, `0x0044BEB5=2/0`, `0x0044BF87=2/0`, `0x0044C195=5/1`, `0x0044C3B5=2/0`, `0x0044C47A=2/0`, `0x0044C6EA=5/1`, `0x0044C8CF=0/0`, `0x0044C95D=5/0` | Building `this` or serviced/produced Techno, repair-and-produce family | Active service/production branches; `/1` calls promote synchronously |
| `0x0044CD12=5/0`, `0x0044D54D=5/0`, `0x0044D571=5/0` | Building `this`, missile/special-building mission family | Conditional active branches |
| `0x0044D6D0=18/0`, `0x0044D6F1=5/0` | Building `this`, adjacent mission helper | Conditional active branches |
| `0x0044DC1E=2/0` | Building/service receiver | Conditional active branch |
| `0x0044DD6B=5/0`, `0x0044DE7E=2/0`, `0x0044E379=5/0` | Building `this` or serviced Techno, dock/service helper | Conditional active branches; first has explicit Commence |
| `0x0044E784=5/0` | Building `this`, late service helper | Conditional active branch |
| `0x004525CC=24/0` | Building `this`, animation/tracker helper | Conditional active branch; explicit Commence follows |
| `0x00458138=15/0` | sold Building/occupant lifecycle receiver | Conditional sell branch |
| `0x00458E8B=5/0`, `0x00459337=5/1` | Building `this`; bunker occupant at Building `+0x2E4` | Active bunker lifecycle; occupant `/1` promotes synchronously |
| `0x004595A6=5/0` | Building `this`, adjacent bunker helper | Conditional active branch |
| `0x004596C8=5/0`, `0x00459807=2/0`, `0x00459820=5/0` | refinery Building and released harvester Unit | Active harvester release/docking branches |
| `0x004725FE=15/0` | captured Techno receiver in capture manager | Conditional active capture path |
| `0x004C73B9=dynamic/0`, `0x004C768C=5/0`, `0x004C7812=16/0` | event-selected Techno receivers | Active synchronized event execution; middle call explicitly Commences |
| `0x004CDA02=2/0` | Fly locomotor's linked Aircraft owner | Active locomotor branch |
| `0x004D3FA7=2/0`, `0x004D416E=11/0`, `0x004D41DF=5/0` | Foot `this`, pathfinding result handling | Active movement/path branches |
| `0x004D4C6B=15/0` | Foot `this`, Capture | Conditional active mission |
| `0x004D51CC=1/0`, `0x004D52A3=17/0` | Foot `this`, Guard | Active Guard branches |
| `0x004D5445=17/0`, `0x004D5497=8/0`, `0x004D5570=2/1` | Foot `this`, Hunt | Active Hunt branches; first two explicitly Ready/Commence; last promotes internally |
| `0x004D6AB9=5/1`, `0x004D6D3B=10/0`, `0x004D6E32=17/0`, `0x004D701F=1/0` | Foot `this`, AreaGuard | Active AreaGuard branches; first promotes internally, second explicitly Commences |
| `0x004D8514=2/0` | Foot `this`, arrival processing | Active arrival callback |
| `0x004D906A=5/0`, `0x004D909F=5/0`, `0x004D91BA=2/0` | Foot `this`, `Receive_Radio` | Conditional radio traffic; explicit Ready/Commence follows relevant calls |
| `0x004DDFC6=11/0` | Foot `this`, Rescue | Conditional AI mission; explicit Commence follows |
| `0x004DF33D=2/1`, `0x004DF36D=1/1`, `0x004DF418=1/1` | Foot `this`, movement/arrival helpers | Active conditional branches; synchronous Queue-owned promotion |
| `0x004DFB5E=9/0`, `0x004DFC8B=7/0` | Foot `this`, adjacent movement helpers | Active conditional branches |
| `0x004DFDE9=8/1`, `0x004DFF2F=8/1`, `0x004E0067=7/1` | Foot `this`, dock/enter search helpers | Active conditional branches; synchronous Queue-owned promotion |
| `0x005014E1=15/0` | Team-released Foot receiver in boundary fragment | Conditional active Team/lifecycle path |
| `0x0050CAAA=2/0` | Foot receiver in team/action helper | Conditional active branch |
| `0x00518116=-1/0`, `0x00518124=5/0` | Infantry `this`, init boundary fragment | Active initialization; first call is a no-op, second is explicitly Commenced |
| `0x00518C3F=15/0` | Infantry `this`, fear handling | Conditional active branch |
| `0x0051A49C=5/0`, `0x0051A732=2/0` | Infantry `this`, `PerCellProcess` | Active per-cell branches |
| `0x0051CD96=dynamic/0` | Infantry `this`, idle dispatch | Active AI branch |
| `0x0051D6C4=2/0` | Infantry `this`, Scatter | Active movement branch |
| `0x0051DFD6=5/0` | Infantry `this`, FireAt | Conditional combat branch |
| `0x0051F449=17/0` | Infantry `this`, mission helper | Conditional active branch |
| `0x0051FEA6=dynamic/0` | Infantry `this`, parsed/action-derived mission | Conditional active helper; explicit Commence follows |
| `0x00521788=5/0`, `0x00521798=15/0` | Infantry `this`, mission helper | Conditional active branches |
| `0x00522D0F=10/0`, `0x00522D26=5/0` | Infantry `this`, adjacent helpers | Conditional active branches |
| `0x00522E9D=5/0`, `0x00522FA6=5/0` | Infantry `this`, late mission helper | Conditional active branches |
| `0x0053B3BD=15/0` | psychically affected Techno receiver | Conditional active psychic path |
| `0x0054B48D=5/1` | Jumpjet locomotor's linked Infantry owner | Active arrival branch; internal promotion, then B8 write and second Ready/Commence retry |
| `0x0054DABB=10/0`, `0x0054E575=1/0`, `0x0054E675=1/0` | Infantry/Foot receivers in adjacent mission and spawn-retreat helpers | Conditional active branches |
| `0x0063860C=28/1` | Foot/Techno receiver in active mission helper | Conditional active branch; synchronous Queue-owned promotion |
| `0x0065DAF4=16/0`, `0x0065DC59=26/0` | newly created/placed Techno receivers | Active object-creation branches; later explicit Commence in owner |
| `0x0065E460=5/0` | newly created Techno receiver | Active object creation; immediate explicit Commence |
| `0x0065E70C=dynamic/0`, `0x0065E964=dynamic/0`, `0x0065EB5E=dynamic/0` | newly created type-specific Techno receivers | Active scenario/object creation; flag register is zero at each call; explicit Commence follows |
| `0x006AF7B0=2/0`, `0x006AF892=2/0`, `0x006AF9AB=2/0`, `0x006AFCBB=2/0` | Slave manager's controlled Techno receivers | Conditional active slave AI |
| `0x006AFEA6=2/0`, `0x006B01E8=19/0` | Unit/Building receiver selected by slave deployment logic | Conditional active slave/deploy path |
| `0x006B0E96=5/0`, `0x006B0ECC=2/0`, `0x006B0FF8=5/0` | returned slave Techno receivers | Conditional active return/release path |
| `0x006B75C4=2/0`, `0x006B7608=2/0`, `0x006B7687=2/0`, `0x006B76D8=2/0`, `0x006B772C=1/0`, `0x006B7788=1/0`, `0x006B785C=2/0`, `0x006B7AFD=1/0` | Spawn manager's spawned/controlled Techno receivers | Conditional active spawn AI |
| `0x006CDDAD=22/0` | superweapon-selected Techno receiver | Conditional active Super launch; explicit Commence follows |
| `0x006DDC52=5/0`, `0x006DDCCD=5/0`, `0x006DDD58=5/0` | trigger-selected Techno receivers | Active scenario trigger actions |
| `0x006E022D=5/0` | Team/trigger-selected Techno receiver | Conditional active script helper |
| `0x006E9099=5/0` | Team member Techno receiver | Active Team target/action assignment |
| `0x006EB6A1=2/0`, `0x006EB745=8/0`, `0x006EB7CB=1/0` | Team convoy member receivers | Conditional active Team AI |
| `0x006EB91A=2/0`, `0x006EBA0A=2/0`, `0x006EBA9A=5/0` | Team member receivers | Conditional active Team helper |
| `0x006EBBAB=2/0`, `0x006EBE24=2/0` | Team convoy members without target | Conditional active Team AI; second explicitly Ready/Commences |
| `0x006EC0BE=2/0`, `0x006EC0F2=5/0` | Team convoy/guard members | Conditional active Team AI |
| `0x006ED3AB=2/0`, `0x006ED49A=7/0` | Team member receivers | Conditional active Team script |
| `0x006ED57D=2/0`, `0x006ED73A=16/0` | Team member receivers | Conditional active Team script |
| `0x006ED88E=2/0`, `0x006ED97A=2/0`, `0x006ED998=2/0`, `0x006EDA57=dynamic/0` | Team member receivers | Conditional active Team script; final mission is branch-selected |
| `0x006EDC44=1/0` | Team member receiver | Conditional active Team script |
| `0x006EF1C9=2/0`, `0x006EF29C=16/0`, `0x006EF3BE=2/0` | Team member receivers | Conditional active Team script; last explicitly Ready/Commences |
| `0x006F49C4=15/0` | Techno `this`, lifecycle/helper | Conditional active branch |
| `0x006F4B0B=1/0` | Techno `this`, `Receive_Radio` | Conditional active radio path |
| `0x006F9F59=5/0`, `0x006F9F68=15/0` | Techno `this`, AI | Active per-object update branches |
| `0x0070156E=5/1` | Techno `this`, `ChangeOwner` | Active ownership transition; synchronous Queue-owned promotion |
| `0x00701DB4=15/0` | Techno `this`, `ReceiveDamage` | Conditional active damage/destruction branch |
| `0x0070870E=dynamic(11-or-21)/0` | Techno `this`, active helper | Conditional branch-selected mission |
| `0x0070D83C=7/1`, `0x0070D864=-1/0` | Techno `this`, TryEnter | Active enter path; first promotes synchronously, second is a no-op |
| `0x0070D95F=2/0` | Techno `this`, adjacent movement helper | Conditional active branch |
| `0x0070F8E0=15/0` | Techno `this`, boundary helper | Conditional active branch |
| `0x0071014E=13/0`, `0x00710386=5/1`, `0x007103D5=15/0` | Techno `this` and linked deployed object, PerformDeploy | Active deploy lifecycle; linked-object `/1` promotes synchronously |
| `0x0073645B=15/0`, `0x007367FE=7/0`, `0x0073697B=2/1` | Unit `this`, AI | Active per-object update branches; last promotes synchronously |
| `0x00736E61=16/0` | Unit `this`, FireAt | Conditional active combat branch |
| `0x0073753E=0/0`, `0x00737ADE=10/0`, `0x00737B2B=5/0` | Unit `this`, `Receive_Radio` | Conditional active radio traffic; second explicitly Ready/Commences |
| `0x0073816E=15/0`, `0x0073833C=15/0`, `0x0073834C=5/0`, `0x007385A1=2/0`, `0x00738668=7/0` | Unit `this`, damage processing | Conditional active damage branches |
| `0x00738D1B=dynamic/0` | Unit `this`, forced scatter | Active movement branch |
| `0x0073957A=5/0`, `0x007396D9=18/0` | Unit `this`, Deploy | Conditional active deploy branches |
| `0x0073A7AE=5/0`, `0x0073AA48=2/0`, `0x0073AA96=11/0`, `0x0073AAEF=10/1`, `0x0073AB29=10/0` | Unit `this`, `PerCellProcess` | Active per-cell branches; `/1` promotes synchronously; nearby explicit Commence calls in Section 6 |
| `0x0073D887=5/0`, `0x0073DBDB=2/0`, `0x0073DCC1=5/0`, `0x0073DE09=15/0`, `0x0073DE18=5/0`, `0x0073DEC1=5/0`, `0x0073E141=10/0`, `0x0073E254=10/0`, `0x0073E330=10/1` | Unit `this`, DeployBuilding | Active mission-state branches; exact explicit Commence positions in Section 6 |
| `0x0073E65A=5/0`, `0x0073E6BC=5/0`, `0x0073EE93=7/0`, `0x0073EED9=20/0`, `0x0073EEEA=15/0`, `0x0073EF71=5/0` | Unit `this`, Harvest | Active harvester state-machine branches |
| `0x00740934=10/0`, `0x007409E9=10/0`, `0x00740A19=16/0`, `0x00740A79=10/0` | Unit `this`, GuardHarvester | Active harvester guard branches |
| `0x00740AF7=5/0` | Unit `this`, Guard | Active mission branch |
| `0x00740F73=7/0` | Unit `this`, Unload | Active mission branch |
| `0x007425B9=7/0`, `0x00742F37=7/0` | Unit `this`, `Set_Destination` family | Active NavCom writer branches |
| `0x00743623=dynamic/0` | newly loaded Unit | Active scenario load; explicit Ready/Commence follows |
| `0x00744053=2/0` | Unit `this`, Scatter | Active movement branch |

The only instruction-search hits intentionally omitted from this ledger are `0x00423C36` (Anim `+0x1E8`), `0x0046826F` (Bullet `+0x1E8`), and `0x0062CFB5` (Particle `+0x1E8`). Their class identities and actual slot targets are proven in Section 3.

## 8. Current Rust Mapping

| Native authority requirement | Current Rust surface | Verdict |
|---|---|---|
| Queue owns a `commence_now` argument and, when nonzero, synchronously performs Ready then conditional Commence | `src/sim/mission/verb.rs:82` exposes `queue_mission(com, mission)` only; no flag, readiness input, or chained Commence | **DRIFT** |
| Queue, Commence, and Restore are used by production owners at their native host positions | `rg` finds `queue_mission`, `commence_queued`, and `restore_mission` only in `verb.rs` unit tests; there are no production callers | **MISSING** |
| Concrete Restore success reinstates saved Target, then for Foot-derived objects saved NavCom | `verb.rs:127` only manipulates `MissionCom`; it has no entity Target/NavCom wrapper | **MISSING** |
| Base Restore preserves dispatch substate/timer as settled by the verb-body report | `verb.rs:127-136` clears `substate` and resets `timer` on restore | **DRIFT** |
| Mission selection changes occur at verified caller/update positions | `retask.rs:72-82` routes new orders through Assign; `retask.rs:89-98` writes current directly; `world/mod.rs:979-1004` tail-projects current/substate; `world/techno_ai.rs:525-558` writes them in the Unit bracket | **PARTIAL / UNMAPPED** to the Queue/Commence/Restore caller ledger |

The existing uncommitted `src/sim/world/techno_ai.rs` change is test-only and was not modified. The search also found direct mission writes inside tests; those are not production authority paths.

## 9. Configuration and Activation Gates

No INI key selects Queue, Commence, or Restore semantics. The calls are engine mechanisms. Individual callsites can still be gated by object class, mission state, locomotor, radio message, Team/trigger script, superweapon, or scenario content as recorded in Sections 5-7. No sampled caller was classified as dormant TS-only code.

## 10. Coverage Ledger

| Required slice | Coverage | Result |
|---|---|---|
| Direct xrefs to three base bodies | all code/data xrefs inspected | COMPLETE |
| Literal `+0x1E8` calls | 259/259 classified: 256 true, 3 rejected | COMPLETE |
| Literal `+0x1EC` calls | 60/60 classified: 59 true, 1 rejected | COMPLETE |
| Literal `+0x1F8` calls | 3/3 classified: 3 true, 0 rejected | COMPLETE |
| Queue mission and flag argument | 256/256 recorded, including dynamic values and zero-valued flag registers | COMPLETE |
| Immediate Queue/Ready/Commence order | base body plus all adjacent explicit sequences inspected | COMPLETE |
| Restore Target/NavCom wrapper order | base, Techno, and Foot wrapper chain plus all callers inspected | COMPLETE |
| Current Rust verb callers/direct writers | repo-wide `src/sim/**/*.rs` search and direct reads | COMPLETE for current tree |

## 11. Open-Question Closure Log

| Question | Resolution and evidence |
|---|---|
| 1. Are direct xrefs caller-complete? | No. They expose override tail calls, while literal-slot search exposes the active virtual population. `get_function_xrefs` plus the 259/60/3 instruction counts. |
| 2. How many true Queue calls exist? | 256; every address is in Section 7. |
| 3. Which Queue-search hits are false? | Anim `0x00423C36`, Bullet `0x0046826F`, Particle `0x0062CFB5`; COL/TD and slot targets in Section 3. |
| 4. Is `0x00424B04` another false Anim hit? | No. Its receiver is the newly allocated Infantry, so it is true Queue. |
| 5. What does Queue flag 1 do? | Synchronous post-write Ready then conditional Commence at `0x005B3621..0x005B3641`. |
| 6. Does Queue flag 0 schedule an automatic later promotion? | No internal promotion exists; subsequent promotion is explicit caller authority or remains pending. Base body plus Sections 6-7. |
| 7. Do callers invoke Commence outside Queue? | Yes: 58 literal virtual calls outside Queue's own `0x005B363B`, spanning update, lifecycle, locomotor, mission, event, Team, and load contexts. Section 6. |
| 8. How many direct virtual Restore callers exist? | Three, at `0x00417706`, `0x00707A4B`, and `0x0070D50D`. Section 5. |
| 9. Does Restore only pop Mission state? | No for concrete Technos: Target and then NavCom are conditionally restored by `0x007013E0` and `0x004D8F80`. |
| 10. Does Queue `-1` clear the queue? | No. It skips the queue write; with flag 0 it is a no-op. `0x005B35FC..0x005B3621`; active callers `0x00518116`, `0x0070D864`. |
| 11. Were any null-receiver calls accepted? | No. Each accepted hit has a receiver flow; four unrelated receiver classes were rejected. |
| 12. Is mission value zero itself a sentinel? | No. Active Queue calls pass `0` at `0x0043C7D4`, `0x0044C8CF`, and `0x0073753E`. |
| 13. Can Ready failure erase the queued mission? | No. Ready runs after the queue write and false skips Commence, leaving queued state. Base Queue order. |
| 14. Are scenario creation/load paths deferred to a global tick tail? | No. The creation/load helpers explicitly Queue and Commence/Ready within their own call stack. `0x0041B3EE`, `0x0065DAF4..0x0065EBE9`, `0x00743623..0x0074363B`. |
| 15. Are same-object update promotions explicit? | Yes. Building Update, Aircraft/Unit AI, per-cell, and locomotor owners invoke Ready/Commence locally. Section 6. |
| 16. Is any accepted call proven TS-only/dormant in standard YR? | No. Content/script/superweapon cases are conditional, not reclassified as dormant. |
| 17. Are the three Rust verbs already authoritative in production? | No. Current-tree references outside their definitions are tests only. Section 8. |

## 12. Adversarial Cases

1. **Queue `-1` must not be modeled as clear.** Both live callsites depend on its no-write behavior; `0x00518116` immediately follows with Queue 5, while `0x0070D864` leaves state unchanged.
2. **Queue flag 1 with Ready false must retain queued state.** The queue mutation precedes Ready and no rollback branch exists at `0x005B35FC..0x005B3641`.
3. **Direct Commence with no queued mission must not fabricate/reset state.** The settled Commence body returns false when no queued value is present; callers are allowed to try.
4. **Failed Restore must not write saved Target or NavCom.** Both derived wrappers gate their setters on the base return; `PointerExpired` and `StopAllTargeting` may still perform their own later cleanup.
5. **A numeric vtable displacement is not type proof.** Treating all `+0x1E8/+0x1EC` calls as Mission verbs would introduce four false callers from three unrelated classes.

## 13. Implementation Handoff

| Verified behavior | Required Rust delta | Surface | Minimal scenario/test | Primary risk |
|---|---|---|---|---|
| Queue owns `commence_now`; `/1` synchronously does Ready then conditional Commence, while `/0` never self-promotes | Replace the two-step test-only abstraction with an authority API that accepts the native flag and category-specific readiness snapshot, preserves queued state on false Ready, and exposes exact return/order | `src/sim/mission/verb.rs`, then bounded production caller adapters | `queue_flag_zero_never_promotes_without_explicit_caller`; `queue_flag_one_runs_ready_then_commence_synchronously`; include false Ready and mission `-1` | Flattening every queue into immediate assignment or end-of-tick promotion changes same-tick visibility/order |
| Concrete Restore success performs base pop -> saved Target restore -> saved NavCom restore, without base timer/substate reset | Correct base Restore mutation and add Rust-native Techno/Foot wrapper authority over entity Target/NavCom fields | `src/sim/mission/verb.rs`, entity target/destination surfaces, lifecycle/target invalidation callers | `restore_success_reinstates_target_then_navcom_without_timer_reset`; failed restore writes neither; StopAllTargeting re-checks Target after restore | A bare `MissionCom` pop loses pursuit/destination intent; unconditional setters resurrect stale links |
| Production callers choose exact host position: local object update, lifecycle, locomotor, mission handler, radio/event/team, and scenario creation/load | Crosswalk each existing direct mission writer to the ledger before wiring; keep synchronous calls at the verified owner rather than introducing one global drain | `src/sim/mission/retask.rs`, `src/sim/world/techno_ai.rs`, `src/sim/world/mod.rs`, relevant lifecycle/locomotor/mission owners | One representative same-tick trace per owner class, including Building Update, Unlimbo, Jumpjet second retry, DeployBuilding state 4, and scenario Unit load | Tail projection can overwrite or delay native host-time commits; broad wiring before caller mapping can double-Commence |

## 14. Do-Not-Do Notes

1. Do not classify a virtual call from its displacement alone; Section 3 proves four false same-offset calls.
2. Do not turn flag-0 Queue into an implicit scheduler job; the binary has no such internal path and many callers explicitly choose if/when to Commence.
3. Do not use `Queue(-1, ...)` as a clearing API; `0x005B35FC` proves it is a write skip.
4. Do not implement concrete Restore as a timer-resetting bare mission pop; wrappers `0x007013E0` and `0x004D8F80` conditionally restore Target/NavCom, and the settled base body does not reset timer/substate.
5. Do not defer Building Update, Unlimbo, locomotor, per-cell, or scenario-load Commence calls to a global tick tail; Section 6 proves synchronous native host positions.

## 15. Remaining Uncertainty

None within the declared direct-plus-literal-virtual static slice. The completeness limit for hypothetical runtime-generated function pointers remains the explicit boundary in Section 2.2; no evidence of such an additional path was found.

## 16. Stale-Document Replacement

The caller paragraph in `docs/research/READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md`, Section "Caller Path Verification", is incomplete as an authority census. Replace it with:

> Direct-xref results identify override tail calls only and are not caller-complete. Executable-wide receiver-proven census finds 259 literal `+0x1E8` calls: 256 Mission-family Queue calls and three unrelated Anim/Bullet/Particle calls; 60 literal `+0x1EC` calls: 59 Commence calls and one unrelated Anim call; and three literal `+0x1F8` calls, all Restore. Queue flag/order, explicit Commence host positions, Restore wrapper order, and the complete ledgers are in `MISSION_QUEUE_COMMENCE_RESTORE_ACTIVE_CALLER_AUTHORITY_GHIDRA_REPORT.md`.

This replacement does not invalidate that document's Ready override bodies; it supersedes only its partial caller statement.

## Sources

- Active retail `gamemd.exe` loaded in Ghidra from `C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe`; read-only decompile, assembly context, instruction search, xrefs, memory, and COL/TypeDescriptor walks performed 2026-07-22.
- `docs/research/MISSIONCLASS_VERB_API_GUARDS_OVERRIDE_RESTORE_SEMANTICS_GHIDRA_REPORT.md` for the previously settled base verb mutation contract; this investigation cold-read only the portions needed to classify caller order.
- `docs/research/READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md` and the newer bounded Ready lifecycle reports for preflight/navigation; caller claims were independently re-censused here.
- Current Rust files named in Section 8, read directly from the working tree on 2026-07-22.

**Status: COMPLETE** for the claimed exhaustive static caller-authority slice.
