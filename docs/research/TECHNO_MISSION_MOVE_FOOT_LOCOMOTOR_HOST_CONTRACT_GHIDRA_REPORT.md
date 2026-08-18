# Techno → Mission Move → Foot Locomotor Host Contract — Ghidra Report

**Date:** 2026-07-20  
**Investigation mode:** `exhaustive-slice`  
**Binary:** retail Yuri's Revenge `gamemd.exe`, PE x86 little-endian, image base `0x00400000`  
**Ghidra policy:** read-only; every query explicitly targeted `program="gamemd.exe"`  
**Checkpoint:** A of `docs/plans/2026-07-20-ground-movement-atomic-flip-readiness-investigation-plan.md`  
**Verdict:** **CHECKPOINT A PASS; PRODUCTION FLIP BLOCKED**

## 1. Overview

This report establishes the exact active-YR host path that an ordinary ground Unit uses around Move mission dispatch and locomotor service. It corrects the two abstractions most likely to produce “movement feels unlike gamemd” drift:

1. The Techno common body is not one generic pre helper, one alive check, dispatch, and one generic post helper. Its two `+0x90` guards are in the middle of distinct native segments.
2. Mission Move's 14–16-count return delay throttles the mission handler only. It does **not** throttle the later Foot locomotor `Process` opportunity.

For a normal Unit that reaches the Foot path, the ordered spine is:

```text
UnitClass::AI
  -> FootClass::AI
     -> TechnoClass::AI_Update
        -> pre prefix through RockingUpdate
        -> guard B (+0x90)
        -> remaining pre-mission work
        -> ++MissionTickCounter (+0xC4)
        -> MissionClass::Mission_Dispatch
           -> ObjectClass::AI
           -> active/timer/health gates
           -> mission 2: Unit +0x22C wrapper
              -> possible FootClass::Mission_Move
           -> store start/scratch/returned delay
        -> passive acquire / bomb / slave / capture work
        -> guard E (+0x90)
        -> remaining Techno post work
     -> Foot post-Techno guard (+0x90)
     -> substantial Foot pre-Process work
     -> five immediate Process gates
     -> ActiveLocomotor ILocomotion::Process (+0x40)
     -> immediate Foot post-Process guard (+0x90)
     -> remaining Foot work
  -> remaining Unit work
```

The binary may skip or supplement this normal spine through class-special and tube paths. Those are enumerated below.

## 2. Scope and parity status

### In scope

- active binary identities and call order for:
  - `TechnoClass::AI_Update @ 0x006F9E50`;
  - `MissionClass::Mission_Dispatch @ 0x005B3060`;
  - mission id `2` and concrete Unit/Infantry/Foot slot bindings;
  - `FootClass::Mission_Move @ 0x004D4200`;
  - the Foot tail through ILocomotion `Process`;
  - Scenario RNG ownership and exact 0..2 rejection behavior;
  - Unit/Infantry active tube precedence and class-special pre-Foot Process paths;
  - the current Rust host mismatch;
  - an inert/test-only future harness boundary.

### Out of scope

- exact Drive speed (`GetCurrentSpeed`), RawTrack metadata, and Drive point consumption;
- complete Walk/Hover/Ship population and miner ownership;
- full tube leaf algorithms;
- lifecycle/effect/cell/cache ownership after Process;
- a runnable `gamemd.exe` oracle and runtime wall-time measurements;
- production authority migration.

A Checkpoint A PASS means this bounded host mechanism is evidence-complete enough for cold review and a later inert harness plan. It is not a parity certification and does not authorize a Unit-only or Drive-only production flip.

## 3. Program and class identity

The active Ghidra program was the retail executable at `<ra2-install>/gamemd.exe`. The open program reported x86 PE, little-endian, 32-bit, image base `0x00400000`.

Load-bearing virtual identities were recovered from complete-object-locator and TypeDescriptor bytes, not local labels:

| Class/interface | Evidence | Relevant slot |
|---|---|---|
| FootClass | COL `0x00800948` -> TypeDescriptor `0x00817B78` -> `.?AVFootClass@@` | vtable `0x007E8C94` + `0x22C` -> `0x004D4200` |
| UnitClass | COL `0x0080CC68` -> TypeDescriptor `0x00842D80` -> `.?AVUnitClass@@` | vtable `0x007F5C70` + `0x22C` -> `0x00740A90` |
| InfantryClass | COL `0x008033B8` -> TypeDescriptor `0x00825508` -> `.?AVInfantryClass@@` | vtable `0x007EB058` + `0x22C` -> `0x0051F660` |
| DriveLocomotionClass ILocomotion subobject | COL `0x007FFDE8` has subobject offset 4; TypeDescriptor `0x00820248` -> `.?AVDriveLocomotionClass@@` | ILocomotion vtable `0x007E7EB0` + `0x40` -> `0x004B0500` |

Evidence calls: `read_memory` at `0x007E8C90`, `0x00800948`, `0x007E8EC0`, `0x007F5C6C`, `0x0080CC68`, `0x007F5E9C`, `0x007EB054`, `0x008033B8`, `0x007EB284`, `0x007E7EAC`, `0x007FFDE8`, and `0x007E7EF0`; `inspect_memory_content` on each TypeDescriptor string.

The local label at `0x00740A90` says `UnitClass__Mission_Guard`. The Unit vtable proves that address is the concrete `+0x22C` Move binding. The label is polluted or role-stale and must not drive implementation.

## 4. Active scheduling context

Newer scheduler research establishes:

- one Main_Tick per reached outer-loop iteration;
- one late live-object pass per Main_Tick;
- a live count-re-reading forward vector walk calling each object's virtual `+0x5C`;
- one normal Foot `Process` opportunity per eligible object turn;
- no independent 15-Hz Drive gate;
- `g_CurrentFrameCounter` increments later in Main_Tick.

Authority: `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`, `PERTICKUPDATE_FULL_ORDERING_LADDER_GHIDRA_REPORT.md`, and `OBJECT_PASS_DRIVE_INVOCATION_SCHEDULING_GHIDRA_REPORT.md`.

This report starts at the concrete Unit/Infantry/Foot `+0x5C` bodies and does not redo those already-verified main-loop anchors.

## 5. TechnoClass::AI_Update is segmented

### 5.1 Ordered contract

Fresh decompile, full disassembly, and targeted assembly contexts establish this load-bearing segmentation:

| Segment | Address anchor | Exact boundary relevant to this contract |
|---|---:|---|
| Pre prefix | `0x006F9E50..0x006FA23B` | Many common-Techno actions. At `0x006FA224` call virtual `+0x298`; if true, call virtual `+0x41C` at `0x006FA236` (RockingUpdate path). |
| Guard B | read `0x006FA23C`; branch `0x006FA244` | Read byte `this+0x90`; zero jumps to function return. |
| Remaining pre | `0x006FA24A..0x006FA645` | Continues hidden/current-cell work, target validation/clears, gattling/deploy/timer work, and pre-acquire cleanup. This is why guard B is not “after all pre work.” |
| Mission counter | `0x006FA646..0x006FA64F` | Read, increment, and store dword `this+0xC4` with ordinary x86 wrapping behavior. |
| Dispatch | `0x006FA655` | Direct call `MissionClass::Mission_Dispatch @ 0x005B3060`. |
| Early post | `0x006FA65A..0x006FA734` | Passive-acquire timer/scan, bomb path, SlaveManager update, CaptureManager update. |
| Guard E | read `0x006FA735`; branch `0x006FA73D` | Read byte `this+0x90`; zero jumps to function return. |
| Remaining post | `0x006FA743..0x006FAFFC` | Self-heal, power heal/drain, virtual `+0x410`, SpawnManager, cloak visibility, target validation, nonbuilding timer/unload state, StageClass, damage-Spark RNG, virtual `+0x4A0`, and EMP recovery paths. |

Evidence: `decompile_function(0x006F9E50)`; full `disassemble_function`; `get_assembly_context` at `0x006FA236`, `0x006FA23C`, `0x006FA646`, `0x006FA655`, `0x006FA65A`, `0x006FA735`, and `0x006FA743`.

### 5.2 Consequences

- Guard B occurs immediately after the conditional Rocking update, but more pre-mission work follows it.
- Guard E occurs only after passive acquisition, bomb work, SlaveManager, and CaptureManager, but more post work follows it.
- A timer-not-due return from Mission Dispatch resumes at `0x006FA65A`. It does not skip early or late Techno post work.
- Replacing this with `pre(); alive(); dispatch(); post();` changes same-turn lifecycle visibility and subsystem order.
- The byte `+0x90` gates are not equivalent by evidence to Rust's `health.current > 0` predicate.

`get_function_callers(0x006F9E50)` returns the active `FootClass::AI @ 0x004DA530` path and `BuildingClass::Update @ 0x0043FB20`. This report follows the Foot caller.

## 6. MissionClass::Mission_Dispatch

### 6.1 Entry and timer gate

Address-order behavior at `0x005B3060`:

1. Call `ObjectClass::AI @ 0x005F3E70` unconditionally (`0x005B3067`).
2. Read byte `this+0x90`; zero returns (`0x005B306C..0x005B3074`).
3. Load `Start = dword +0xC8` and `Delay = dword +0xD0`.
4. If `Start != -1`:
   - `Elapsed = g_CurrentFrameCounter - Start`;
   - if signed `Elapsed >= Delay`, handler is due;
   - otherwise `Remaining = Delay - Elapsed`.
5. If the resulting delay/remaining value is nonzero, return.
6. Only on the due path, read signed dword `Health = +0x6C` and require `Health > 0`.
7. Read dword `CurrentMission = +0xAC`. Mission values `0..31` enter the jump table; mission id `2` reaches `0x005B334E` and calls virtual `+0x22C`.
8. After the handler returns:
   - `+0xC8 = g_CurrentFrameCounter`;
   - `+0xCC =` an uninitialized stack-local/scratch dword;
   - `+0xD0 = EAX`, the handler's returned delay.

Evidence: `decompile_function(0x005B3060)`; `get_assembly_context` at `0x005B3060`, `0x005B30A1`, `0x005B30A7`, and `0x005B334E`; `read_memory(0x005B34E8,32)`.

### 6.2 Exact due/not-due outcomes

| Path | ObjectClass::AI | Handler | Health read | `+0xC8/+0xCC/+0xD0` rewrite | Return site |
|---|---:|---:|---:|---:|---|
| `+0x90 == 0` after Object AI | yes | no | no | no | Techno `0x006FA65A` |
| Timer not due | yes | no | no | no | Techno `0x006FA65A` |
| Timer due, health <= 0 | yes | no | yes | no | Techno `0x006FA65A` |
| Timer due, mission 2 | yes | Unit/Infantry/Foot binding | yes | yes | Techno `0x006FA65A` |

`get_function_callers(0x005B3060)` returns only `TechnoClass::AI_Update @ 0x006F9E50`.

The exact full-program consumer set for `+0xCC` is deferred. It is not read by this timer gate; prior MissionClass reports classify it as dead/uninitialized scratch, but this checkpoint does not upgrade that negative claim to a new exhaustive proof.

## 7. Mission 2 concrete dispatch

### 7.1 UnitClass wrapper at 0x00740A90

For an ordinary Unit/MCV, Mission Dispatch does **not** call Foot Move directly.

Exact wrapper order:

1. Read `this+0x6E0` and clear byte `this+0x6D2 = 0`.
2. If any byte `+0x6E0`, `+0x6E1`, or `+0x6E2` is nonzero:
   - call virtual `+0x1E8(5,0)`;
   - return 1;
   - do not call Foot Move and do not consume Move jitter RNG.
3. Otherwise form embedded receiver `this+0x350` and call `0x004A51D0`.
4. If that returns zero, call `0x004A5240` on the embedded receiver with the two dwords at `Type(+0x6C4)+0x3C8/+0x3CC`.
5. Tail-call `FootClass::Mission_Move @ 0x004D4200` and return its value.

`0x004A51D0` returns 1 only when embedded bytes `+0x18` and `+0x19` are both zero. `0x004A5240` arms/restarts that embedded timer-like object, multiplies the supplied double by 900, converts with ftol, and writes its frame/duration state. Exact semantic field names are not required for the host contract and remain deferred.

Evidence: Unit RTTI/vtable bytes; `decompile_function` and `disassemble_function` at `0x00740A90`; `decompile_function` at `0x004A51D0` and `0x004A5240`.

### 7.2 Infantry override at 0x0051F660

Infantry's concrete Move binding:

1. Read dword `this+0x6C4`.
2. If it is not in `{0x1B,0x1C,0x1D,0x1E}`, tail-call Foot Move.
3. In that set:
   - call virtual `+0x3C`;
   - concrete Infantry `+0x3C` is `0x006F9DC0` and returns `this+0x21C`;
   - pass that House/owner pointer to `HouseClass::IsPlayerControl @ 0x0050B730`.
4. Player-controlled: virtual `+0x480(0,1)`, return 1.
5. Non-player, signed `*(Type(+0x6C0)+0x6C4) < 0`:
   - virtual `+0x558(0x1F,0,0)`;
   - return `*(*(Type+0xE3C)+0x460)`.
6. Otherwise tail-call Foot Move.

Evidence: Infantry RTTI/vtable bytes; `read_memory(0x007EB094,8)`; `decompile_function` at `0x0051F660`, `0x006F9DC0`, and `0x0050B730`.

The old report's animation-update interpretation and byte-width claim were wrong. Numeric state meanings are deferred rather than guessed.

### 7.3 Foot base handler at 0x004D4200

Exact body:

```text
if NavCom(+0x5A4) == null {
    if ActiveLocomotor(+0x674) == null:
        Assert(E_POINTER)                 // then continues

    moving = ActiveLocomotor.vtable[+0x10]()  // Is_Moving

    if moving == 0 && QueuedMission(+0xB4) == -1:
        this.vtable[+0x484](0, 1)         // arrival hook
        return 1
}

entry = GetMissionTimerEntry(CurrentMission(+0xAC))
base = ftol(entry.Rate(+0x10) * 900.0)
jitter = Scenario(+0x218).RandomRanged(0, 2)
return base + jitter
```

Branch table:

| NavCom | Locomotor result | Queued mission | Outcome | Scenario RNG |
|---|---|---|---|---:|
| non-null | not polled | any | timer formula | one `RandomRanged` API call |
| null | moving | any | timer formula | one API call |
| null | stopped | not `-1` | timer formula | one API call |
| null | stopped | `-1` | arrival `+0x484(0,1)`, return 1 | none |
| null | null pointer | invariant violation | Assert then dereference path | not safely defined |

Evidence: `decompile_function` and full `disassemble_function` at `0x004D4200`; `get_assembly_context` at `0x004D423A` and `0x004D4266`; `get_function_callees(0x004D4200)`.

ILocomotion slot `+0x10` is `Is_Moving`. `Is_Moving_Now` is protocol slot `+0x80`. For Drive, `+0x10 -> 0x004AFB80`. The old `0x004B6610` association was not the concrete Drive mapping.

## 8. Rate, ftol, and Scenario RNG

### 8.1 Mission-control data

`GetMissionTimerEntry @ 0x005B3A00` returns `g_MissionControl_Array + CurrentMission * 8 dwords`, a 32-byte stride.

`MissionControlClass::Read_INI @ 0x005B3760` reads:

- booleans `NoThreat`, `Zombie`, `Recruitable`, `Paralyzed`, `Retaliate`, and `Scatter`;
- double `Rate` at `+0x10`;
- double `AARate` at `+0x18`, copying Rate when AARate is zero.

Effective YR `rulesmd.ini` says:

```ini
[Move]
Rate=.016
```

The same value exists in base `rules.ini`. INI configures metadata; it does not specify the host call order, timer restart, virtual binding, locomotor gate, manager ordering, or RNG ownership.

### 8.2 Conversion

`read_memory(0x007E27F8,8)` yields `0000000000208c40`, the IEEE-754 double `900.0`.

`Math::ftol @ 0x007C5F00`:

- saves the x87 control word;
- if it differs from global `0x0E7F`, loads `0x0E7F`;
- converts with `FISTP qword`.

Rounding-control bits in `0x0E7F` select toward zero. Thus:

```text
ftol(.016 * 900.0) = ftol(14.4) = 14
```

### 8.3 RNG ownership and consumption

The Move callsite:

```text
MOV EAX,[0x00A8B230]
LEA ECX,[EAX+0x218]
PUSH 2
PUSH 0
CALL 0x0065C7E0
```

`Init_Game` calls `ScenarioClass` construction at `0x006832C0` and stores the returned pointer into `0x00A8B230`. The constructor forms `this+0x218` and calls `Random::Seed @ 0x0065C6D0`. Therefore this is the Scenario singleton's embedded deterministic RandomClass.

`RandomRanged(0,2)` makes one API call. Internally, with the stream enabled, it advances the XOR-lag generator, masks to two bits, rejects candidate 3, and repeats until the result is <=2. Exact raw advancement is therefore `1 + number_of_rejected_3s`, not always one.

This RNG work happens only on the Foot timer branch. The arrival-return-1 branch and Unit wrapper's three-byte branch consume no Move-jitter RNG.

## 9. Return to Foot and locomotor Process

### 9.1 Foot post-Techno guard

`FootClass::AI @ 0x004DA530` directly calls Techno at `0x004DA539`, then immediately:

- reads byte `this+0x90` at `0x004DA53E`;
- the `JZ` instruction at `0x004DA548` targets the shared function epilogue at `0x004DAF00`;
- only a surviving object continues;
- `this+0x6B3` is then cleared at `0x004DA54E`, establishing that the arrival byte is a per-pass/reentrancy guard, not a permanent “already arrived” flag.

Evidence: `decompile_function(0x004DA530)`; `get_assembly_context` at `0x004DA539` and `0x004DA53E`.

Fresh caller/epilogue spot-check (2026-07-20): Techno guard B's `JZ` at `0x006FA244` and guard E's `JZ` at `0x006FA73D` both target the Techno epilogue at `0x006FAFFD`, which returns at `0x006FB004`. Control therefore resumes after Foot's direct call at `0x004DA539` and executes the immediate `+0x90` read at `0x004DA53E` even when either Techno guard caused the return. In an ordered trace, a failed guard B or guard E is consequently followed by the Foot post-Techno guard read/fail, but by no later Foot pre-Process work. Evidence: read-only `get_assembly_context(program="gamemd.exe")` at `0x006FA244`, `0x006FA73D`, `0x006FAFFD`, `0x004DA539`, `0x004DA53E`, `0x004DA548`, and `0x004DAF00`.

### 9.2 Substantial pre-Process work

Foot does not call Process immediately after the post-Techno guard. It performs multiple systems first, including conditional damage/heal work, visual/sound state, ILocomotion queries, fog-border work, and periodic cell actions. Some conditional branches have their own alive return.

An implementation that dispatches then immediately moves omits this same-object ordering window.

### 9.3 Five immediate gates

At `0x004DA806..0x004DA856`, Process is eligible only if all of these pass:

| # | Exact test | Failure destination |
|---:|---|---|
| 1 | dword `this+0x674 != 0` | `0x004DAA01` |
| 2 | byte `this+0x3CD == 0` | `0x004DAA01` |
| 3 | byte `this+0x8D == 0` | `0x004DAA01` |
| 4 | dword `this+0x2A8 == 0` **or** byte `TypeReturnedByVirtual(+0x84)+0x692 != 0` | `0x004DAA01` |
| 5 | byte `this+0x81 == 0` | `0x004DAA01` |

A failed gate skips Process but does not return from Foot; execution joins later Foot work at `0x004DAA01`.

Evidence: `get_assembly_context(0x004DA806)`, `get_assembly_context(0x004DA850)`, and full Foot decompile.

### 9.4 Process and immediate post guard

After the five gates:

1. Foot redundantly checks `this+0x674` and asserts `E_POINTER` if null.
2. It loads the active locomotor and calls ILocomotion virtual `+0x40` at `0x004DA877`.
3. It immediately compares byte `this+0x90` at `0x004DA87A`.
4. The `JZ` instruction at `0x004DA880` targets the shared Foot epilogue at `0x004DAF00`; no later Foot work runs.

For DriveLocomotionClass's ILocomotion subobject, `+0x40` is `DriveLocomotionClass::Process @ 0x004B0500`. Evidence: Drive RTTI/COL and `read_memory(0x007E7EF0,8)`.

This is one Process call per eligible passage through this site. It is independent of whether Mission Dispatch called Move on this turn.

## 10. Class-special and tube precedence

These paths prevent a future harness from assuming every Unit/Infantry visit is exactly one normal Techno→Foot→Process chain.

### 10.1 UnitClass::AI at 0x007360C0

| Path | Exact evidence | Effect on normal spine |
|---|---|---|
| Class-special pre-Foot Process | At `0x007362B5..0x007362EA`, virtual predicates `+0x1D8` and `+0x1D4` plus byte `+0x27C` can cause an active-locomotor `+0x40` call; `+0x90` is checked immediately at `0x007362ED`. | Process can occur before the normal Foot call. Subsequent predicates are re-evaluated, so do not assume it is always mutually exclusive with later Foot Process without the exact fixture state. |
| Countdown/lifecycle early return | In the normal-side branch, dword `+0x6D8 != -1` is incremented and compared to Type `+0xE38`; expiry performs death/explosion/lifecycle calls and returns. | No tube, Techno, Mission, or normal Foot Process after the return. |
| Active tube | Signed byte `this+0x684 >= 0` at `0x007363A4..0x007363AC` calls `UnitClass::TubeMovement @ 0x007359F0`, calls virtual `+0x4A0(0)`, and returns. | Definite bypass of Foot/Techno/Mission/ordinary locomotor Process for that object turn. |
| Normal Foot | Signed tube byte <0 and other normal-side gates continue to direct `FootClass::AI @ 0x004DA530` at `0x0073647B`. | Enters the contract in sections 5–9. |
| Special-state cleanup branch | Re-evaluated virtual `+0x1D8/+0x1D4` predicates can route to cleanup/cancel behavior instead of the normal-side branch. | May bypass Foot after the class-special Process. |

Evidence: `decompile_function(0x007360C0)`; `get_assembly_context` at `0x007362EA`, `0x007363A4`, and `0x0073647B`; `search_instructions` within Unit AI for `CALL [..+0x40]` and direct `0x004DA530`.

Fresh post-Foot caller-boundary check (2026-07-20): the normal Unit path calls Foot at `0x0073647B` and resumes unconditionally at `0x00736480`. Unit then executes an intervening tail slice beginning with Type bytes `+0xD2F/+0xD30`, optional `0x007468C0`, and the `this+0x3CD` branch/body. The next Unit `this+0x90` read is delayed until `0x007365BB`, with `JZ 0x00736981` at `0x007365C3`; there is no immediate Unit post-Foot active guard. Consequently, a Foot epilogue return caused by Techno guard B/E, Foot's own post-Techno guard, or the immediate post-Process guard still returns into this intervening Unit-tail region. A bounded inert host harness should stop at Foot return unless that Unit tail and delayed guard are separately modeled. `+0x90 == 0` here describes inactive-state control flow; it is not by itself proof of physical deletion or store removal. Evidence: read-only `get_assembly_context(program="gamemd.exe")` at `0x0073647B`, `0x00736480`, `0x007365BB`, and `0x007365C3`.

Active low-bridge tube movement is a YR mechanism, not dormant TS subterranean locomotion. The tube leaf body owns position/cell/exit/arrival effects; those internals are Checkpoint C/D work.

### 10.2 InfantryClass::AI at 0x0051BAB0

The plan's `0x0051BF00` anchor is mid-body, not a function entry. Infantry's `+0x5C` vtable entry is `0x0051BAB0`.

| Path | Exact evidence | Effect |
|---|---|---|
| Active tube first | Signed byte `this+0x684 >= 0` at `0x0051BAB8..0x0051BAC0` calls `0x0051B350`, virtual `+0x4A0(0)`, and returns. | Bypasses all Foot/Techno/Mission/ordinary Walk Process work. |
| Class-special pre-Foot Process | Virtual `+0x1D8` or (`+0x1D4` and byte `+0x27C`) can call active locomotor `+0x40` at `0x0051BBC0`, followed by immediate `+0x90` check. | May service locomotion before the normal Foot call. |
| Special-state return | A later `+0x1D4` branch can clear/cancel state and return before Foot. | Bypasses normal Foot. |
| Normal Foot | Direct `0x004DA530` at `0x0051BC9F`, followed immediately by another `+0x90` check at `0x0051BCA4`. | Enters common contract, then guards Infantry-only tail work. |

Evidence: Infantry RTTI/vtable `+0x5C -> 0x0051BAB0`; `decompile_function(0x0051BAB0)` and `0x0051B350`; `get_assembly_context` at `0x0051BAB0`, `0x0051BBC0`, and `0x0051BC9F`.

Exact semantic names for virtual `+0x1D4/+0x1D8` and byte `+0x27C` are deliberately not guessed. Their control-flow role is enough to constrain an ordinary fixture and to prevent a false one-call invariant.

## 11. INI and data integration

The four stock INI/art files do not encode the native host order.

Load-bearing direct data:

- `rulesmd.ini:30439-30453` says mission behavior is generally hard-coded while characteristics can be overridden.
- `rulesmd.ini:30484-30485` supplies `[Move] Rate=.016`.
- `rules.ini:22635-22636` supplies the same base value.
- Per-type `Speed`, `Locomotor`, `MovementZone`, `ROT`, acceleration, SpeedType, terrain percentages, and related movement keys feed later locomotor/speed/path logic, not this call-order contract.
- Rocker, Ivan/bomb, slave, and mind-control keys configure content/effect gates. There are no `BombManager=`, `SlaveManager=`, or `CaptureManager=` scheduling keys.
- Art files supply presentation sequences/rates and do not define Techno/Mission/Foot host gates.

Therefore no INI-based simplification can justify moving manager work, changing guard placement, or tying Process cadence to `[Move] Rate`.

## 12. Current Rust mapping

### 12.1 What exists

Current Rust has useful scaffolding:

- `Simulation::object_ai_stage` at `src/sim/world/techno_ai.rs:68`;
- live-order walk and category shell at `:278-355`;
- Unit bracket `unit_techno_bracket` at `:525`;
- mission-family classifier `unit_dispatch_family` at `src/sim/mission/dispatch.rs:50`;
- projected `GameEntity::derived_mission` at `src/sim/game_entity.rs:559`;
- global ground movement `tick_movement_with_grids` at `src/sim/movement/movement_tick.rs:831`;
- world order calls object host, then snapshots live order and calls global movement at `src/sim/world/mod.rs:2215-2240`.

### 12.2 Exact disparities

| Native requirement | Current Rust | Verdict |
|---|---|---|
| Techno prefix through Rocking, guard B, more pre | `techno_common_pre` is empty; one health-based check follows | DRIFT |
| Native `+0x90` byte semantics | `is_alive()` is `health.current > 0`; `is_active()` is `!dying` | DRIFT/UNCHECKED mapping |
| Mission Dispatch calls Object AI and timer-gates a handler | Rust increments a counter and copies `derived_mission` | DRIFT |
| Concrete Unit `+0x22C` wrapper | No executed wrapper | DRIFT |
| Foot Move branches and Scenario jitter | No executed handler | DRIFT |
| Handler return writes start/scratch/delay | No native-equivalent dispatch rewrite | DRIFT |
| Passive acquire/bomb/slave/capture before guard E | Not present in `techno_common_post` | DRIFT |
| Actual guard E before remaining post | The bracket performs no second alive check | DRIFT |
| Remaining Techno post | `techno_common_post` implements only the damage-Spark-related slice | DRIFT |
| Foot post-Techno guard and pre-Process systems | Not represented as the object's next host segment | DRIFT |
| Five immediate Process gates | Global movement categorization differs | DRIFT |
| Process interleaved in the same live-object turn | Movement is a later global snapshot pass | DRIFT |
| Immediate post-Process `+0x90` guard | No equivalent host guard at the native point | DRIFT |
| Tube/class-special precedence | Tube and forced-track work are bundled in global movement | DRIFT/INCOMPLETE |

Current comments that describe the bracket as `pre -> alive -> dispatch -> alive -> post` are stale relative to the binary and even to the current function body: the second alive check is not executed.

The global movement pass also bundles drive re-aim, tubes, forced tracks, pending arrivals, blocker snapshots, mover loops, deferred crush effects, finished-arrival handling, formation sync, Hover work, and other postlude behavior. Checkpoint A does not identify a safe production extraction boundary for that bundle.

## 13. Parity-safe next boundary

The only authorized next implementation target is an **inert/test-only ordinary-Drive harness** after cold review.

### Required isolation

- compile only under `#[cfg(test)]` or a similarly inert debug-only proof surface;
- operate on cloned fixtures or trace-only records;
- do not mutate authoritative production simulation state;
- do not remove or skip any path in `tick_movement_with_grids`;
- do not change the production scheduler;
- do not consume production RNG;
- do not add a Unit-only/Drive-only production dispatch;
- stop the bounded trace at Foot return; do not invent an immediate Unit post-Foot guard or claim the intervening Unit tail is absent;
- do not stage, commit, or activate without a separate user-approved implementation plan.

### Ordinary Unit fixture preconditions

The harness must make its assumptions explicit:

- category Unit with Unit `+0x22C` semantics;
- `CurrentMission == Move`;
- timer due;
- `+0x90 != 0` at each relevant native gate;
- health `+0x6C > 0`;
- Unit `+0x6E0/+0x6E1/+0x6E2 == 0`;
- tube byte `+0x684 < 0`;
- no expired `+0x6D8` lifecycle countdown;
- class-special predicates/fields set so the desired normal Foot path is reached;
- active Drive locomotor;
- all five Foot Process gates pass unless a test deliberately flips one.

These are fixture constraints, not permission to hardcode production assumptions.

## 14. Harness acceptance matrix

A later plan should create exact ordered-event assertions for at least:

| Test | Expected event facts |
|---|---|
| Timer not due | Object AI runs; no Unit wrapper, no Move handler, no Move jitter, no timer rewrite; Techno early post, guard E, late post, Foot pre, and eligible Drive Process still occur. |
| Timer due, NavCom live | Unit wrapper reaches Foot; exactly one Move `RandomRanged(0,2)` API call; timer writes returned 14/15/16; later Process is independently eligible. |
| Timer due, stopped, no queue | Arrival `+0x484(0,1)`; return/write delay 1; no Move jitter; later Foot Process opportunity still follows its own gates. |
| Timer due, stopped, queued | No arrival; timer formula and Scenario RNG execute. |
| Unit wrapper byte set | Queue virtual `+0x1E8(5,0)`, return/write 1; no Foot Move and no Move jitter. |
| Guard B kills/exits | Techno returns through its epilogue; Foot immediately reads `+0x90` and fails its post-Techno guard. No remaining Techno pre, dispatch, Techno post, Foot pre-Process work, or Process occurs. |
| Timer handler or early post clears `+0x90` before guard E | Early post through CaptureManager completes; guard E returns through the Techno epilogue; Foot immediately reads `+0x90` and fails. No late Techno post, Foot pre-Process work, or Process occurs. |
| One Foot gate fails | No ordinary Process; execution joins later Foot work at `0x004DAA01`. |
| Drive Process clears `+0x90` | The `JZ` at `0x004DA880` targets the shared Foot epilogue at `0x004DAF00`; no later Foot work. Unit then resumes at `0x00736480`; the bounded harness stops at Foot return rather than modeling that Unit tail. |
| Unit active tube, with the class-special pre-Foot Process predicates disabled in this fixture | Tube leaf and `+0x4A0(0)`; no Foot/Techno/Mission/ordinary Process. If those class-special predicates are enabled, the earlier `+0x40` Process event must be allowed before the tube test. |
| Infantry active tube | Infantry tube leaf and `+0x4A0(0)`; no Foot/Techno/Mission/ordinary Process. |
| RNG rejection | A raw candidate 3 causes another raw Scenario state step within the single API call. |

Rust-only hashes can ratchet regressions but cannot certify gamemd parity. A future parity claim still requires a gamemd-derived executable check or exhaustive proof.

## 15. Adversarial review

1. **Could `[Move] Rate=.016` be the reason vehicles only physically advance every 14–16 counts?**  
   No. It gates the handler in Mission Dispatch; Foot Process is a later independent site.

2. **Could an ordinary Unit dispatch directly to `0x004D4200` because it inherits Foot?**  
   No. Unit vtable bytes route `+0x22C` to `0x00740A90` first.

3. **Could `0x00A8B230+0x218` be treated as cosmetic/noncritical RNG?**  
   No. Init_Game stores a constructed Scenario object there, and its constructor seeds the embedded RandomClass at `+0x218`.

4. **Could one common pre helper, one health check, dispatch, and one post helper preserve the native order?**  
   No. Guard B is mid-pre after RockingUpdate; guard E is mid-post after passive/bomb/slave/capture, with significant work on both far sides.

5. **Could every Unit/Infantry turn be asserted to call Process exactly once?**  
   No. Active tube paths call their leaf and return without ordinary Process; class-special paths can call `+0x40` before Foot, and their later predicates determine whether Foot is also reached.

6. **Could a vehicle-only production flip be called parity-safe because ordinary Drive is now understood?**  
   No. Complete ground population, exact speed/track metadata, lifecycle/effect ownership, and native oracle gates remain unresolved; the approved design requires an atomic population boundary.

## 16. Open Questions Log

No Checkpoint-A question is silently open.

- **[RESOLVED] OQ-01:** Active executable identity and image base.
- **[RESOLVED] OQ-02:** Techno function boundary and active Foot caller.
- **[RESOLVED] OQ-03:** Guard B location and exact branch.
- **[RESOLVED] OQ-04:** Work that remains after guard B before dispatch, at contract granularity.
- **[RESOLVED] OQ-05:** `+0xC4` increment immediately before direct Mission Dispatch.
- **[RESOLVED] OQ-06:** Guard E location after passive/bomb/slave/capture.
- **[RESOLVED] OQ-07:** Significant late post work after guard E.
- **[RESOLVED] OQ-08:** Mission Dispatch always calls Object AI first.
- **[RESOLVED] OQ-09:** Timer-not-due and due predicates.
- **[RESOLVED] OQ-10:** Health read occurs only on due path.
- **[RESOLVED] OQ-11:** Mission id 2 selects virtual `+0x22C`.
- **[RESOLVED] OQ-12:** Handler return storage at `+0xC8/+0xCC/+0xD0`.
- **[RESOLVED] OQ-13:** Foot, Unit, and Infantry concrete Move bindings.
- **[RESOLVED] OQ-14:** Unit wrapper order and no-RNG early return.
- **[RESOLVED] OQ-15:** Infantry owner-return slot and dword state width.
- **[RESOLVED] OQ-16:** Foot NavCom/moving/queue/arrival branch matrix.
- **[RESOLVED] OQ-17:** Rate table stride, Rate offset, and stock value.
- **[RESOLVED] OQ-18:** 900.0 constant and ftol rounding mode.
- **[RESOLVED] OQ-19:** Scenario RNG receiver identity.
- **[RESOLVED] OQ-20:** One API call versus one-or-more raw RNG advances.
- **[RESOLVED] OQ-21:** Foot post-Techno guard.
- **[RESOLVED] OQ-22:** Exact five immediate Process gates.
- **[RESOLVED] OQ-23:** Drive `+0x40 -> 0x004B0500`.
- **[RESOLVED] OQ-24:** Immediate post-Process `+0x90` guard.
- **[RESOLVED] OQ-25:** Unit and Infantry tube precedence.
- **[RESOLVED] OQ-26:** Infantry AI true entry is `0x0051BAB0`, not `0x0051BF00`.
- **[RESOLVED] OQ-27:** No separate 15-Hz Drive scheduler gate.
- **[RESOLVED] OQ-28:** Current Rust host/body/global-movement disparity.
- **[DEFERRED] OQ-29:** Exact semantic names and full active-state matrix for virtual `+0x1D4/+0x1D8`, `+0x27C`, and Unit `+0x278`. Reason: control flow is sufficient for an ordinary fixture; full population precedence belongs to Checkpoint C.
- **[DEFERRED] OQ-30:** Exact semantic names for Unit `+0x6E0..+0x6E2` and Infantry states `0x1B..0x1E`. Reason: numeric gates are exact; naming needs focused class-state audits.
- **[DEFERRED] OQ-31:** Exhaustive read/observability proof for Mission `+0xCC`. Reason: not a host/timer input; revisit before byte-exact MissionClass storage.
- **[DEFERRED] OQ-32:** Full OnArrival tail and `+0x544` identity. Reason: arrival/effect ownership is Checkpoint D.
- **[DEFERRED] OQ-33:** Exact GetCurrentSpeed and RawTrack contracts. Reason: Checkpoint B.
- **[DEFERRED] OQ-34:** Walk/Hover/Ship/miner/forced-track/full tube population. Reason: Checkpoint C.
- **[DEFERRED] OQ-35:** Lifecycle, occupancy, crush, scatter, cache, and postlude ownership. Reason: Checkpoint D.
- **[DEFERRED] OQ-36:** Native executable oracle and wall-time/runtime measurements. Reason: Checkpoint E.

## 17. Coverage ledger

### Deep binary reads

- `TechnoClass::AI_Update @ 0x006F9E50`
- `MissionClass::Mission_Dispatch @ 0x005B3060`
- `FootClass::Mission_Move @ 0x004D4200`
- `FootClass::AI @ 0x004DA530`
- `UnitClass::AI @ 0x007360C0`
- `InfantryClass::AI @ 0x0051BAB0`
- `UnitClass::TubeMovement @ 0x007359F0`
- Infantry tube leaf `0x0051B350`
- Unit Move wrapper `0x00740A90`
- Infantry Move override `0x0051F660`
- `ObjectClass::AI @ 0x005F3E70`
- `MissionClass::GetMissionTimerEntry @ 0x005B3A00`
- `MissionControlClass::Read_INI @ 0x005B3760`
- `RandomClass::RandomRanged @ 0x0065C7E0`
- `Math::ftol @ 0x007C5F00`
- Scenario constructor `0x006832C0`
- Drive Is_Moving `0x004AFB80`
- Foot OnArrival `0x004D82B0`
- Unit embedded tracker helpers `0x004A51D0` and `0x004A5240`
- Infantry owner-return slot `0x006F9DC0`
- `HouseClass::IsPlayerControl @ 0x0050B730`

### Vtable/RTTI checks

- Foot, Unit, Infantry complete-object locators and TypeDescriptors
- Foot/Unit/Infantry `+0x22C` targets
- Unit/Infantry `+0x5C` AI targets
- Infantry `+0x3C` target
- Drive ILocomotion COL/subobject identity
- Drive ILocomotion `+0x10` and `+0x40` targets
- Foot `+0x484` arrival target

### Source/data reads

- current object host, Unit bracket, classifier, derived mission, movement pass, and world tick order;
- `rulesmd.ini`, `rules.ini`, `artmd.ini`, `art.ini`;
- current scheduling, Techno-body, MissionClass, ILocomotion, tube, and movement reports.

### Zero-add and cold checks

After synthesis:

1. Cold spot-check A re-read Foot Move decompile, its assembly branch, and all three class `+0x22C` bytes.
2. Cold spot-check B re-read Techno guards B/E, Foot post-Techno/Process gates, and the immediate post-Process guard.
3. Cold spot-check C re-read Mission Dispatch timer due/not-due assembly and Scenario RNG ownership callsites.
4. The zero-add pass found no new Checkpoint-A question; new uncertainties were assigned explicitly to B–E or focused field-name audits.

## 18. Remaining blockers and handoff

Checkpoint A closes the stale scheduling/15-Hz misconception and the exact ordinary Unit host ordering prerequisite.

The atomic production flip remains blocked by:

- exact `FootClass::GetCurrentSpeed`, including widths, signedness, conversion, modifiers, and stock fixtures;
- RawTrack/TurnTrack metadata and every active initializer;
- complete active ground population: miner, Infantry/Walk, Hover, Ship, forced track, and active tubes;
- same-object lifecycle/effect ownership: arrivals, cell/occupancy, crush, scatter, sound, gates/factories/walls, caches, formation behavior, deferred deletes;
- full `+0xCC` storage observability if MissionClass byte state is migrated;
- executable gamemd-derived oracle/runtime evidence;
- a mechanically proven atomic extraction/removal boundary across the complete Phase-1 ground population.

Next action after cold review: write a separate implementation plan for the inert cloned-fixture ordinary-Drive harness only. Production remains unchanged.

## 19. Sources

### Direct Ghidra evidence

All calls used `program="gamemd.exe"`.

- `decompile_function`: `0x006F9E50`, `0x005B3060`, `0x004D4200`, `0x004DA530`, `0x007360C0`, `0x0051BAB0`, `0x007359F0`, `0x0051B350`, `0x00740A90`, `0x0051F660`, `0x005F3E70`, `0x005B3A00`, `0x005B3760`, `0x0065C7E0`, `0x007C5F00`, `0x004AFB80`, `0x004D82B0`, `0x004A51D0`, `0x004A5240`, `0x006F9DC0`, `0x0050B730`.
- `disassemble_function`: `0x006F9E50`, `0x005B3060`, `0x004D4200`, `0x004DA530`, `0x00740A90`.
- `get_assembly_context` anchors cited throughout sections 5–10.
- `read_memory` / `inspect_memory_content` for RTTI, vtables, `900.0`, ftol control word, and dispatch jump table.
- `get_function_callers` / `get_function_callees` for Techno, Mission Dispatch, Foot, and Move.
- `search_instructions` within Unit and Infantry AI for direct locomotor `+0x40` and Foot calls.

### Research/data/source corroboration

- `docs/research/FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md` — corrected companion audit
- `docs/research/TECHNOCLASS_AI_UPDATE_BODY_GHIDRA_REPORT.md`
- `docs/research/OBJECT_PASS_DRIVE_INVOCATION_SCHEDULING_GHIDRA_REPORT.md`
- `docs/research/LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`
- `docs/research/PERTICKUPDATE_FULL_ORDERING_LADDER_GHIDRA_REPORT.md`
- `docs/research/ILOCOMOTION_COM_PROTOCOL_SPEC.md`
- `docs/research/MISSIONCLASS_VERB_API_GUARDS_OVERRIDE_RESTORE_SEMANTICS_GHIDRA_REPORT.md`
- `docs/research/FOOTCLASS_RECEIVE_RADIO_0X12_MOVE_FIELDS_NAVCOM_GHIDRA_REPORT.md`
- `ini/rulesmd.ini:30439-30515`
- `ini/rules.ini:22635-22636`
- `src/sim/world/techno_ai.rs:68-655`
- `src/sim/world/mod.rs:2202-2240`
- `src/sim/game_entity.rs:559-598`
- `src/sim/game_entity.rs:845-858`
- `src/sim/mission/dispatch.rs:1-66`
- `src/sim/movement/movement_tick.rs:831-1855`
