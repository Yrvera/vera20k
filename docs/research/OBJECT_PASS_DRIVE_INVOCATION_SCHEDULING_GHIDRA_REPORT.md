# Object-Pass / Drive Invocation Scheduling — Ghidra Research Report

**Address(es):** `Main_Game @ 0x0048CCC0`, `Main_Tick @ 0x0055D360`, per-tick live-vector pass `@ 0x0055AFB0`, `UnitClass::AI @ 0x007360C0`, `FootClass::AI @ 0x004DA530`, `TechnoClass::AI_Update @ 0x006F9E50`, `Mission_Dispatch @ 0x005B3060`, `DriveLocomotionClass::Process @ 0x004B0500`, `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`, local wait helper `@ 0x0055E160`, modal pump `@ 0x00623120`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** active standard-YR scheduling from the outer game loop through one reached `Main_Tick`, the main live-object pass, a normal stock Unit/Foot mission bracket, the active locomotor `Process` call, DriveTrack invocation count, local-skirmish pacing, late frame-counter placement, and bounded pause/replay/network contrasts.  
**Non-Scope:** Drive point-body arithmetic, the exact `GetCurrentSpeed` formula, RawTrack metadata, A*/path selection, complete class-specific AI bodies, full network/replay protocol, save serialization, and measured retail runtime cadence.  
**Confidence:** **High** for static call count, order, gates, vtable identity, residual-only retry, lack of a separate Drive cadence gate, frame-counter placement, local speed-byte source, and offline modal behavior. **Medium** for this coverage-map as a whole because realized retail wall timing, exact focus-transition ownership, and first post-load Drive state were not dynamically measured.  
**Active in YR:** **Yes.** Stock `[AMCV]` selects the verified Drive locomotor CLSID, and the traced path is the normal live vehicle path rather than a TS-only locomotor branch.  
**Parity verdict for current Rust scheduling:** **DRIFT.** This is a mechanism verdict, not a frequency/severity downgrade.  
**Completion boundary:** The static invocation-scheduling slice is complete. Overall locomotion/pathfinding parity is **not certified** by this report.

## 0. Working Notes and Duplication Check

Target question: when, how often, and in what object-relative order does active `gamemd.exe` invoke a normal ground vehicle's Drive locomotor?

The recent implementation contract and approved design left this exact cadence as their load-bearing blocker. Existing research already covered Drive point stepping and same-tick retry behavior, so this investigation extended only the missing owner schedule. It did not redo RawTrack, `Can_Enter_Cell`, A*, or point-delta formulas.

Required evidence was:

1. the outer owner and number of `Main_Tick` calls per outer iteration;
2. the number and placement of the main per-tick live-object pass;
3. concrete Unit and Drive vtable identities rather than trust in local labels;
4. mission-dispatch versus locomotor order;
5. every immediate gate on the locomotor call;
6. whether Drive has an independent frame/15 Hz movement gate;
7. the local speed-byte source and timer-domain budget;
8. current Rust phase and cadence evidence;
9. an independent cold verification of the load-bearing claims.

The cold verifier found no contradiction. A final zero-add read of `Main_Tick`, the live-vector loop, `FootClass::AI`, Drive `Process`, DriveTrack's budget mask, and the modal pump added no new in-scope question.

## 1. Verdict and Native Ordered Spine

For a normal eligible stock Drive vehicle, native ground movement is coupled to that object's live-object turn:

```text
one Main_Game outer iteration
  -> one Main_Tick
     -> one reached main live-object pass
        -> current object vtable +0x5C
           -> UnitClass::AI normal path
              -> FootClass::AI
                 -> TechnoClass::AI_Update
                    -> +0xC4 increment
                    -> Mission_Dispatch
                 -> active ILocomotion vtable +0x40
                    -> DriveLocomotionClass::Process
                       -> zero, one, or two DriveTrack calls according to Drive state
     -> Network_ServiceLoop
     -> guarded g_CurrentFrameCounter increment
     -> local pacing wait
```

There is **no independent 15 Hz service** between the live-object turn and Drive motion. When the Unit/Foot path and concrete gates permit it, Foot calls the current locomotor `Process` once in that object turn. Drive `Process` can then call DriveTrack twice in the same invocation, but the retry call masks fresh speed and uses residual budget only.

Game speed changes the wall-clock spacing of `Main_Tick` calls. It does not change the amount of Drive work admitted within one reached object turn. At the stock YR local-skirmish stored value `GameSpeed=1`, the timer-domain budget is one 16 ms bucket, giving a static nominal ceiling near 62.5 reached ticks per second. Actual retail throughput and jitter remain runtime-unmeasured.

This directly invalidates the current Rust split in which Drive speed state is updated on scheduled simulation steps while active DriveTrack point consumption is admitted through a separate nominal 15 Hz gate.

Binary evidence: `disassemble_function(address="0x0048CCC0")`, `disassemble_function(address="0x0055D360")`, `disassemble_function(address="0x0055AFB0")`, `disassemble_function(address="0x007360C0")`, `disassemble_function(address="0x004DA530")`, `disassemble_function(address="0x006F9E50")`, `disassemble_function(address="0x004B0500")`, and `disassemble_function(address="0x004B0F20")` against active `gamemd.exe`.

## 2. Identity, Layout, and Call-Slot Proof

Local names were treated as navigation hints. The load-bearing class identities were re-proved from vtable bytes and RTTI.

| Surface | Verified bytes / role | Evidence |
|---|---|---|
| Unit vtable start | `0x007F5C70` | `read_memory(address="0x007F5C6C", length=0x64)` |
| Unit vtable `+0x5C` | dword at `0x007F5CCC` = `0x007360C0` | same read |
| Unit vtable `[-1]` | COL pointer `0x0080CC68` | same read |
| Unit COL type descriptor | pointer `0x00842D80`; bytes decode `.?AVUnitClass@@` | `read_memory(0x0080CC68, 0x20)` and `read_memory(0x00842D80, 0x30)` |
| Foot active locomotor | pointer at `Foot+0x674` | `disassemble_function(0x004DA530)`, `0x004DA806..0x004DA877` |
| ILocomotion Process slot | active interface vtable `+0x40` | call at `0x004DA877` |
| Drive ILocomotion vtable start | `0x007E7EB0` | `read_memory(address="0x007E7EAC", length=0x50)` |
| Drive vtable `+0x40` | dword at `0x007E7EF0` = `0x004B0500` | same read |
| Drive vtable `[-1]` | COL pointer `0x007FFDE8`, COL subobject offset `4` | `read_memory(0x007FFDE8, 0x20)` |
| Drive COL type descriptor | pointer `0x00820248`; bytes decode `.?AVDriveLocomotionClass@@` | `read_memory(0x00820248, 0x40)` |
| Techno mission counter | dword at `Techno+0xC4` | increment/store `0x006FA646..0x006FA64F` |
| Drive residual budget | signed integer at Drive `+0x4C` | loaded before mask/add at `0x004B1284`, stored by DriveTrack |
| Drive track index / point | Drive `+0x58` / `+0x5C` | Drive `Process` and DriveTrack decompile |

The Drive COL's offset `4` matters: the locomotor interface pointer is a subobject pointer, and Drive `Process` adjusts it (`LEA EDI,[ESI-4]`) before accessing Drive-owned state. A Rust-native implementation need not copy COM layout, but it must preserve the verified call semantics and state owner.

## 3. Core Scheduling Logic

### 3.1 Outer owner: one `Main_Tick` per `Main_Game` iteration

`Main_Game @ 0x0048CCC0` has one direct `CALL Main_Tick` at `0x0048CE8A`. The result is followed by state-machine/session handling, and the outer back-edge at `0x0048CEA8` returns to `0x0048CE8A`.

There is no loop inside one `Main_Tick` that repeats the main live-object pass to catch up. The local wait helper can service network/input/render-related work while waiting, but it does not call the full live-vector function at `0x0055AFB0` again.

Evidence: `disassemble_function(address="0x0048CCC0")`, especially `0x0048CE8A..0x0048CEA8`; `disassemble_function(address="0x0055E160")`.

### 3.2 `Main_Tick` pass placement and whole-pass gates

The normal reached `Main_Tick` path contains exactly one direct call to the per-tick live-vector function:

```asm
0055DC99  MOV  ECX,0x87F778
0055DC9E  CALL 0x0055AFB0
```

Important whole-pass cases:

| Condition | Main live-object pass | Late frame increment | Evidence |
|---|---:|---:|---|
| `g_GameActive == 0` at entry | no | no | `0x0055D360..0x0055D371` |
| `g_GameRunning == 0` | local modes `0`/`5` remain in the pre-work wait loop; nonlocal modes service once and leave that loop | no local object work until running resumes | `decompile_function(0x0055D360)` entry loop |
| Scenario `+0x62C != 0` early-return path | no | no | Main_Tick decompile; branch ending near `0x0055D860` |
| `g_GameState != 0`, if Main_Tick is entered | yes | normally yes | gameplay block gate `0x0055D878..0x0055D901`, late call/increment |
| replay record/playback bits, on paths that continue | yes | normally yes | replay block converges on `0x0055DC9E` |
| four late session-end flags clear | already ran | yes | `0x0055DE4F..0x0055DE81` |
| any of the four late session-end flags set | already ran | no; terminal exit also bypasses the pacing wait and remaining tail helpers | same range, branch to `0x0055DEC8` |

The Scenario `+0x62C` branch services messages, network, queued events, tactical/render, and wait, then returns before both the live-object pass and counter increment. Separately, any of the four late session-end flags branches to the terminal exit at `0x0055DEC8` after the object pass: it bypasses the counter increment, `FUN_0055E160`, and the remaining normal tail helpers.

Evidence: `decompile_function(address="0x0055D360")` and `disassemble_function(address="0x0055D360")`, especially `0x0055D878..0x0055D918`, `0x0055DC99..0x0055DCA3`, and `0x0055DE4F..0x0055DE9A`.

### 3.3 Main live-vector mutation semantics

The main object loop is `0x0055B5FB..0x0055B619`:

```asm
0055B5FF  XOR  ESI,ESI                    ; index = 0
0055B601  MOV  EAX,[EDI+0x10]             ; current count
0055B608  MOV  EAX,[EDI+0x4]              ; current data pointer
0055B60B  MOV  ECX,[EAX+ESI*4]            ; load current item now
0055B610  CALL dword ptr [EDX+0x5C]
0055B613  MOV  EAX,[EDI+0x10]             ; reload live count
0055B616  INC  ESI
0055B617  CMP  ESI,EAX
0055B619  JL   0x0055B608
```

Consequences:

- this is a forward live pass, not a frozen object snapshot;
- tail-appended objects can be reached later in the same pass;
- compacting removal can shift the next object into the processed index while the index still increments, skipping that shifted object for this pass;
- the current pointer and count are re-read around each call;
- one object turn can mutate state visible to later object turns.

Evidence: `disassemble_function(address="0x0055AFB0")`, main loop `0x0055B5FB..0x0055B619`. The preceding `+0x5C` loop at `0x0055B5D9..0x0055B5E8` belongs to a different collection and is not the main object loop used for this conclusion.

### 3.4 Eligible Unit order: mission before locomotion

On the normal Unit path that reaches common Foot behavior:

1. `UnitClass::AI @ 0x007360C0` calls `FootClass::AI @ 0x004DA530` at `0x0073647B`.
2. Foot immediately calls `TechnoClass::AI_Update @ 0x006F9E50` at `0x004DA539`.
3. Techno increments `Techno+0xC4` at `0x006FA646..0x006FA64F`.
4. Techno calls `Mission_Dispatch @ 0x005B3060` at `0x006FA655`.
5. Control returns through the remainder of Foot.
6. Foot calls the active locomotor interface `+0x40` at `0x004DA877`.
7. Foot immediately checks owner alive byte `+0x90` at `0x004DA87A`.

Thus the same object's mission dispatch completes before its locomotor `Process`. Native does not first dispatch every Unit mission and then begin a separate whole-world ground movement phase.

Unit has special paths before `0x0073647B`; this report does not claim that every Unit state reaches Foot. The ordering claim applies to the normal eligible stock vehicle path that does.

Evidence: `disassemble_function(0x007360C0)`, `disassemble_function(0x004DA530)`, and `disassemble_function(0x006F9E50)`.

### 3.5 Concrete immediate gates on the Foot locomotor call

The locomotor call at `0x004DA877` is reached only when:

| Gate | Required value | Assembly |
|---|---:|---|
| active locomotor pointer `Foot+0x674` | non-null | `0x004DA806..0x004DA814` |
| byte `Foot+0x3CD` | zero | `0x004DA81A..0x004DA820` |
| byte `Foot+0x8D` | zero | `0x004DA826..0x004DA82C` |
| if dword `Foot+0x2A8` is nonzero | type byte `+0x692` must be nonzero | `0x004DA832..0x004DA84A` |
| byte `Foot+0x81` | zero | `0x004DA850..0x004DA856` |

Foot then reloads `+0x674` and makes one `vtable+0x40` call. No frame-counter modulo test gates this call. Foot has an earlier modulo-controlled upkeep/effect block, but its paths reconverge before the locomotor site; it is not a locomotion cadence gate.

Evidence: `disassemble_function(address="0x004DA530")`, `0x004DA806..0x004DA880`; independently cold-verified from the same assembly.

### 3.6 Drive `Process`: two call sites do not mean two frames

Drive `Process @ 0x004B0500` has two static DriveTrack call sites:

- `0x004B0576 -> 0x004B0F20`: active-track entry with argument `0`;
- `0x004B0AAA -> 0x004B0F20`: shared later call reached with argument `0` on no-active-track start, or argument `1` on same-invocation retry after a completed active track and `Process_Movement`.

One Drive `Process` invocation can therefore execute:

```text
Process_Drive_Track(0)
  -> current track completes
  -> Process_Movement selects/installs continuation
  -> Process_Drive_Track(1) in the same object turn
```

The retry call is not a second fresh movement frame. DriveTrack still updates speed-fraction state and calls owner `GetCurrentSpeed` at `0x004B1274`, then the argument mask at `0x004B127A..0x004B128D` zeros that fresh value when the retry argument is nonzero. `ADD EDX,EDI @ 0x004B1295` adds only the stored Drive `+0x4C` residual to the masked value.

This detail is stronger than merely saying “retry budget is zero”: the native retry still traverses DriveTrack's pre-budget speed-state work before masking the fresh integer budget.

Evidence: `disassemble_function(0x004B0500)`, call contexts `0x004B0562..0x004B058C`, `0x004B063D..0x004B0667`, and `0x004B0A6B..0x004B0ABC`; `decompile_function(0x004B0F20)`; `disassemble_function(0x004B0F20)`, `0x004B1261..0x004B129A`.

### 3.7 No independent Drive movement cadence gate

Neither Foot's locomotor call nor either DriveTrack call is controlled by a frame-rate divisor, `% 4`, or a 15 Hz admission counter.

The only `g_CurrentFrameCounter % 10` sequence in Drive `Process` is at `0x004B079D..0x004B07AC`, after DriveTrack work. It gates a secondary animation/effect allocation path, not movement. It cannot support a claim that Drive translation runs at one quarter of the live-object pass rate.

Evidence: full `disassemble_function(0x004B0500)` plus cold verification of all frame/modulo instructions in the function; full `disassemble_function(0x004DA530)` around the locomotor site.

## 4. Frame Counter and Local Pacing

### 4.1 Counter placement

On the normal path, the live-object pass runs at `0x0055DC9E`. After later work and `Network_ServiceLoop @ 0x0055DE4A`, four flags are tested. Only when they are all clear does `g_CurrentFrameCounter` increment at `0x0055DE73..0x0055DE81`. The wait helper runs afterward at `0x0055DE9A`.

Therefore:

- mission and Drive observe the pre-increment frame value for that object pass;
- the normal reached path has one live-object pass and one later counter increment;
- session-end flags can allow object/Drive work and then take a terminal late exit that suppresses the increment, pacing wait, and remaining normal tail helpers;
- the counter is not a separately synthesized 15 Hz clock.

Evidence: `disassemble_function(address="0x0055D360")`.

### 4.2 Local timer-domain mapping

`GetRadarTimer @ 0x006C8C40` is:

```asm
CALL [timeGetTime]
SHR  EAX,0x4
RET
```

One returned timer unit is therefore a 16 ms bucket. For local modes, Main_Tick records the start bucket and copies the live stored speed byte to the wait budget (`0x0055D79E..0x0055D7BC`). The helper at `0x0055E160` subtracts elapsed buckets and waits/services until the budget is exhausted.

| Stored speed byte | Required timer-bucket progress | Nominal duration | Static nominal ceiling |
|---:|---:|---:|---:|
| `0` | none | free-run / workload-limited | not statically capped by this wait |
| `1` | 1 bucket | 16 ms | ~62.5 reached ticks/s |
| `2` | 2 buckets | 32 ms | ~31.25/s |
| `3` | 3 buckets | 48 ms | ~20.83/s |
| `4` | 4 buckets | 64 ms | ~15.63/s |
| `5` | 5 buckets | 80 ms | 12.5/s |
| `6` | 6 buckets | 96 ms | ~10.42/s |

These are timer-domain budgets, not measured retail FPS. Bucket quantization, work already consumed, OS scheduling, rendering, and service-loop behavior can change individual intervals and achieved throughput.

Evidence: `disassemble_function(0x006C8C40)`, `disassemble_function(0x0055D360)`, and `disassemble_function(0x0055E160)`.

## 5. INI and Active-YR Inputs

| Input | Active stock value | Scheduling role | Evidence |
|---|---|---|---|
| `[MultiplayerDialogSettings] GameSpeed` | base `rules.ini:2506` = `0`; YR patch `rulesmd.ini:3026` = `1` | default local-skirmish stored wait byte | INI plus readers below |
| `[AMCV] Speed` | `4` | Drive speed input; exact `GetCurrentSpeed` conversion is outside this report | `rulesmd.ini:6980` |
| `[AMCV] ROT` | `5` | Drive facing input, not an invocation cadence gate | `rulesmd.ini:6986` |
| `[AMCV] Locomotor` | `{4A582741-9839-11d1-B709-00A024DDAFD1}` | selects Drive locomotor | `rulesmd.ini:6998` |
| `[AMCV] MovementZone` | `Normal` | normal active ground vehicle context | `rulesmd.ini:7000` |

The binary source chain is:

1. `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0` reads `GameSpeed` into `Rules+0x14A0`.
2. `Load_Game_Rules @ 0x0052CD70` copies `Rules+0x14A0` to `DAT_00A8B268` at `0x0052D177..0x0052D17D`.
3. `SessionClass__ReadSkirmishSettings @ 0x00697F10` also reads per-skirmish `GameSpeed` with `Rules+0x14A0` as fallback.
4. pregame setup paths copy `DAT_00A8B268` to live speed byte `DAT_00A8EB60` (for example `FUN_005E7460`).
5. standard local skirmish `Main_Tick` reads `DAT_00A8EB60` at `0x0055D79E` and stores it as the wait budget.

The mode-0 force-to-2 branch in `FUN_0069BAB0` does not apply to standard skirmish mode 5.

Evidence: `decompile_function(0x00671EA0)`, `decompile_function(0x00697F10)`, `disassemble_function(0x0052CD70)`, `decompile_function(0x005E7460)`, `decompile_function(0x0069BAB0)`, and `get_xrefs_to(address="0x00A8EB60")`.

## 6. Pause, Replay, Network, and Boundary Matrix

| Scenario | Verified scheduling result | Scope note |
|---|---|---|
| Normal local skirmish | one reached live-object pass per Main_Tick, local speed-byte wait after late increment | primary active slice |
| Entered Main_Tick with `g_GameState != 0` | normal gameplay input/map/render block is skipped; late live-object pass still runs | body-level fact |
| Offline campaign/skirmish Options modal | modal pump modes `0`/`5` call network service and never call Main_Tick | actual offline pause behavior |
| LAN/WOL Options modal | can call Main_Tick if blocker/reentrancy gates permit | bounded network contrast |
| Replay record/playback path | replay work converges on the same single late live-object pass | full replay protocol out of scope |
| Scenario delay `+0x62C` | services/render/waits, then returns before pass and increment | verified exception |
| Late session-end flag | pass already ran; terminal branch suppresses the counter increment, pacing wait, and remaining normal tail helpers | verified exception |
| Save/load first resumed Drive call | not established | deferred to serialization trace |
| Exact focus-loss transition owner | not established | deferred to broader app-state trace |

The apparent pause contradiction is therefore resolved by call ownership: “`g_GameState != 0` still reaches PerTick” is true only if `Main_Tick` is entered. Actual standard offline Options loops on `0x00623120`, whose mode-0/mode-5 branches do not call `Main_Tick` at all.

Evidence: `disassemble_function(0x00623120)`, `0x00623120..0x00623161`; `disassemble_function(0x004E1D00)`, Options back-edge `0x004E1D70..0x004E1D98`; `decompile_function(0x0055D360)`.

## 7. Adversarial Edge Cases

1. **Can both DriveTrack executions update position/track state in one object turn?** Yes. Foot invokes the locomotor once, and Drive may make a same-invocation retry using stored residual. The verified distinction is that this is not a second object turn and the retry receives no fresh integer budget.
2. **Can an eligible Unit miss its locomotor call?** Yes. A null locomotor, the concrete Foot byte gates, special Unit early paths, or death before the site can prevent it.
3. **Can an object added mid-pass run immediately?** A tail append can run later in the same live pass because count is re-read.
4. **Can removal change who runs?** Yes. Forward index plus compaction can skip the successor shifted into the processed slot.
5. **Does ESC/Options prove paused Main_Tick still moves units offline?** No. The offline modal pump does not enter Main_Tick.
6. **Does lag make one native Main_Tick execute several live-object passes?** No. The outer loop may iterate again, but one Main_Tick contains one direct pass call.
7. **Does `GameSpeed` multiply Drive budget inside one call?** No. It controls the local wall-clock wait budget.
8. **Does `g_CurrentFrameCounter % 10` throttle Drive motion?** No. That test is downstream secondary effect logic.
9. **Can the last active tick move objects without advancing the frame counter?** Yes. A late session-end flag can branch to terminal exit after PerTick has already run; that exit also skips the pacing wait and remaining normal tail helpers.
10. **Can no-active-track Drive state begin a track immediately?** Yes. `Process_Movement` can install a track and the shared later DriveTrack call can process it in the same Drive invocation.

## 8. Current Rust Comparison

Current Rust contains useful live-order and Drive scaffolding, but the native mechanism is not yet present.

| Surface | Current Rust evidence | Native comparison | Verdict |
|---|---|---|---|
| App base clock | `src/app_types.rs:24..45`: `SIM_TICK_HZ=45`, integer `SIM_TICK_MS=22`; speed 1 maps to 63 TPS | speed 1 is directionally close to native's static nominal 62.5 timer-bucket ceiling, but native speed 0 is free-run and achieved pacing is unmeasured | DRIFT / partly aligned |
| Catch-up | `src/app_sim_tick.rs:36..39`, `:547..606`, `:1213..1233`: one host update may schedule several `advance_tick` calls | native `Main_Game` performs one Main_Tick per outer iteration, with state-machine work between iterations | DRIFT |
| Offline modal | `src/app_sim_tick.rs:410..478`: current session is hardwired Skirmish; paused offline sim is suppressed | matches actual offline Options pump freeze; network modal branch is not live yet | aligned for current offline mode only |
| Object mission host | `src/sim/world/mod.rs:2213..2232`; `src/sim/world/techno_ai.rs:282..354`, `:518..558` | live walk exists and eligible non-miner Unit mission is committed in it | partial alignment |
| Ground movement owner | `src/sim/world/mod.rs:2234..2262`: snapshot then separate whole-world ground movement phase | native mission and locomotor are interleaved within each eligible object's turn | DRIFT |
| Drive Process surface | `src/sim/movement/drive_locomotion.rs:26..35`: shell returns a marker; actual work is elsewhere | native `Process` owns the ordered Drive state machine | DRIFT |
| Global Drive work | `src/sim/movement/movement_tick.rs:913..934`, `:1147..1214`, `:1340..1357` | speed fraction and translation happen in later mover phase, not at Foot's call site | DRIFT |
| Active track cadence | `src/sim/movement/movement_step.rs:42..69`, `:739..772`: explicit 15 Hz admission every three 45 Hz subticks | no native independent 15 Hz Drive movement gate exists | DRIFT |
| Retry | `src/sim/movement/movement_step.rs:667..725`, `:801..858`: newly selected track can retry with zero fresh budget | residual-only budget direction is useful, but native also executes DriveTrack's pre-budget speed-state work on retry | partial / DRIFT |
| Binary frame | `src/sim/world/mod.rs:2172..2181`: synthesized `binary_frame=(total_sim_ms*15)/1000` | native counter increments once after each normal reached Main_Tick, not on a separate 15 Hz clock | DRIFT |

Two scheduler differences are especially likely to produce the player's visible “nothing like original” result:

1. Rust finishes the hosted mission decisions for the whole live vector before any ordinary ground mover advances. Native completes mission then locomotion for object A before it starts object B's turn. This changes same-pass state visibility.
2. Rust updates Drive speed state on scheduled mover iterations but admits active track points only on its separate 15 Hz clock. Native couples both through every eligible Drive invocation.

At Rust's configured speed-1 target of about 63 simulation steps per wall second, the explicit three-subtick gate admits fresh active-track processing only about 21 times per nominal second. Native offers a DriveTrack opportunity on every eligible reached object pass, statically near 62.5/s at default local pacing. This comparison is about invocation opportunities and state ordering; it does **not** assert a threefold displacement error because the exact native `GetCurrentSpeed` conversion is outside this report.

Inference, clearly separated from direct evidence: delayed curve-point/cell-crossing commits and whole-vector mission-before-movement ordering can change when reservations, occupancy, and dynamic blockers become visible. That can make pathfinding decisions diverge even when the A* implementation itself is unchanged.

## 9. Coverage Ledger and Tiny-Detail Findings

| Area / detail | Status | Evidence / result | Remaining work |
|---|---|---|---|
| Active program identity | verified | `get_current_program_info`: retail `gamemd.exe`, image base `0x00400000` | none |
| Main outer loop | verified | one call `0x0048CE8A`, back-edge `0x0048CEA8` | none |
| Main_Tick direct pass count | verified | exactly one `CALL 0x0055AFB0` at `0x0055DC9E` | none |
| Scenario early return | verified | skips pass and increment | none |
| Live vector forward order | verified | `0x0055B5FF..0x0055B619` | none |
| Count re-read after body | verified | `0x0055B613` | none |
| Current pointer loaded immediately before call | verified | `0x0055B608..0x0055B610` | none |
| Unit vtable identity | verified | RTTI/COL plus slot bytes | none |
| Normal Unit-to-Foot call | verified | `0x0073647B` | special Unit paths outside slice |
| Foot-to-Techno first call | verified | `0x004DA539` | none for order |
| `+0xC4` before mission dispatch | verified | `0x006FA646..0x006FA655` | mission bodies outside slice |
| Mission before locomotor | verified | `0x006FA655` precedes `0x004DA877` through caller return | none |
| Foot locomotor pointer/slot | verified | `Foot+0x674`, vtable `+0x40` | none |
| Immediate Foot gates | verified | five concrete gate groups listed above | semantic names for every byte not needed |
| Foot post-Process alive check | verified | byte `+0x90` at `0x004DA87A` | none |
| Drive vtable identity | verified | RTTI/COL offset 4 plus slot bytes | none |
| Active-track first DriveTrack call | verified | `0x004B0576`, arg 0 | none |
| No-active-track movement then DriveTrack | verified | `0x004B0A79` then `0x004B0AAA`, arg 0 | path selection details outside slice |
| Same-call retry | verified | `0x004B0647`, push 1 at `0x004B0665`, shared call `0x004B0AAA` | none |
| Retry recomputes then masks speed | verified | `0x004B1274..0x004B1295` | exact speed formula outside slice |
| Drive `%10` role | verified | downstream effect path, not movement | effect allocation details outside slice |
| Counter after Drive/object pass | verified | pass `0x0055DC9E`, increment `0x0055DE73..81` | none |
| Terminal late exit after pass | verified | four flags `0x0055DE4F..71` branch to `0x0055DEC8`, skipping increment/wait/remaining tail | flag owner details outside slice |
| Local timer bucket | verified | `timeGetTime() >> 4` | runtime jitter unmeasured |
| Stock local speed source | verified | YR INI -> Rules -> session/UI live byte | user-modified settings vary |
| Offline modal no-entry | verified | pump mode 0/5 branches | none |
| Replay convergence | touched/verified for scheduling | replay block reaches single late pass | protocol out of scope |
| Network pacing | touched | separate branch / negotiated divisor | full protocol out of scope |
| Current Rust phase order | verified from source | object host then movement snapshot | tree can move; anchors current 2026-07-20 |
| Current Rust 15 Hz Drive gate | verified from source | explicit constants and delay function | implementation not performed |
| Current Rust synthetic 15 Hz frame | verified from source | late formula | implementation not performed |
| Exact focus transition | deferred | broader app-state owner required | dedicated trace |
| First post-load Drive invocation | deferred | serialization owner required | dedicated trace |

Static scheduling completion status: **COMPLETE for claimed scope**. Parity certification status: **not certified**; no executable gamemd-vs-Rust oracle was run.

## 10. Open Questions — Final State

- `[RESOLVED] OQ-01 — Is Main_Tick the active repeating owner?` Yes; `Main_Game` calls it once per outer iteration at `0x0048CE8A`.
- `[RESOLVED] OQ-02 — Which gates prevent the pass entirely?` Inactive game; the local mode-0/mode-5 pre-work running wait; and Scenario `+0x62C` early return. Nonlocal modes can leave the `g_GameRunning==0` loop after one service and continue. Outer modal ownership can also prevent Main_Tick entry.
- `[RESOLVED] OQ-03 — Does g_GameState pause skip PerTick?` An entered Main_Tick still reaches it; actual offline Options does not enter Main_Tick.
- `[RESOLVED] OQ-04 — Does replay skip the pass?` No on continuing replay paths; replay work converges on the one late pass.
- `[RESOLVED] OQ-05 — Does Scenario+0x62C skip pass and increment?` Yes.
- `[RESOLVED] OQ-06 — Exactly one pass call per reached Main_Tick?` Yes, one direct call at `0x0055DC9E`.
- `[RESOLVED] OQ-07 — What are live-vector mutation semantics?` Forward index, current pointer load, post-call live-count reload, no index repair.
- `[RESOLVED] OQ-08 — Which `+0x5C` implementation owns a stock Unit turn?` `0x007360C0`, proved by Unit RTTI/COL and vtable bytes.
- `[RESOLVED] OQ-09 — Exact Unit/Foot/Techno order?` Unit normal path -> Foot -> Techno.
- `[RESOLVED] OQ-10 — Mission before locomotor?` Yes, `+0xC4` and `Mission_Dispatch` complete first.
- `[RESOLVED] OQ-11 — Locomotor field and slot?` `Foot+0x674`, interface vtable `+0x40`.
- `[RESOLVED] OQ-12 — Does stock Drive resolve to `0x004B0500`?` Yes, proved by Drive RTTI/COL, vtable bytes, and stock CLSID.
- `[RESOLVED] OQ-13 — More than one Foot locomotor call per object turn?` One call site; current pointer is called once when gates pass.
- `[RESOLVED] OQ-14 — Can DriveTrack run twice?` Yes; the second call is same-invocation residual-only budget.
- `[RESOLVED] OQ-15 — Null/idle/no-track behavior?` Null skips Foot call; present idle Drive still receives Process and can return internally; no-track can select/start in the same call.
- `[RESOLVED] OQ-16 — Any Foot/Drive movement cadence gate?` No; observed modulo checks govern other work.
- `[RESOLVED] OQ-17 — Counter placement?` After object/Drive work and network service.
- `[RESOLVED] OQ-18 — One pass to one increment?` On the normal path yes; late flags can take a terminal exit after the pass, skipping the increment, pacing wait, and remaining normal tail helpers.
- `[RESOLVED] OQ-19 — Stock local speed source/default?` YR `[MultiplayerDialogSettings] GameSpeed=1` through Rules/session/live byte.
- `[RESOLVED] OQ-20 — Rate or per-call content?` Rate: wall spacing changes; Drive work per reached invocation does not.
- `[RESOLVED] OQ-21 — Exact timer mapping?` Speed `0` free-run; `1..6` require `1..6` 16 ms bucket advances.
- `[RESOLVED] OQ-22 — Network/replay pacing?` Separate branches; not authority for standard offline skirmish pacing.
- `[RESOLVED] OQ-23 — Native catch-up passes inside one Main_Tick?` No.
- `[DEFERRED] OQ-24 — Exact first/last/focus transition matrix.` First normal entry, terminal late session-end exit, and offline modal no-entry are resolved; exact Win32 focus-loss ownership requires broader app-state research.
- `[RESOLVED] OQ-25 — Current Rust phase order?` Whole live object mission host, then separate snapshot-based ground movement, then late synthetic frame/tick commit.
- `[RESOLVED] OQ-26 — Current Rust pause behavior?` Offline hardwired Skirmish freeze matches offline Options; network modal behavior is not live.
- `[RESOLVED] OQ-27 — Does Rust 45 Hz / 15 Hz Drive gate match native cadence?` No verified native 15 Hz Drive gate exists.
- `[RESOLVED] OQ-28 — Stale/contradictory docs?` Yes; listed in section 12.
- `[DEFERRED] OQ-29 — First resumed Drive invocation after save/load.` Requires a serialization/restore trace of Drive residual, speed fraction, track index/point, and active locomotor ownership.
- `[RESOLVED] OQ-30 — TS legacy contamination?` No for the load-bearing stock Unit/Drive chain.

Final tally: **28 resolved, 2 deferred, 0 open**.

## 11. Implementation Handoff

No Rust implementation was performed. The approved per-object-host approach is supported by this investigation.

| Verified contract | Current delta | Required effect in a future patch | Acceptance scenario | Risk / do not do |
|---|---|---|---|---|
| One eligible object's mission dispatch precedes that same object's locomotor Process inside the live pass | all hosted Unit missions complete before global ground movement | absorb authoritative ordinary ground Drive processing into the per-object host, preserving live-order mutation semantics | object A moves/reserves before object B's mission/locomotion turn; tail append and compaction fixtures match native visitation | do not keep a whole-world mission phase followed by a whole-world Drive phase |
| Foot calls current locomotor once behind concrete gates | Drive shell is only a marker; global mover filter owns behavior | express equivalent Rust-native gate/dispatch ownership at the per-object site | null/deployed/limbo/special-gate fixtures invoke or skip exactly once | do not introduce COM/vtables; preserve semantics, not C++ architecture |
| Every active-track or newly selected-track Drive path can advance DriveTrack in the same Process invocation; no independent 15 Hz gate | explicit three-subtick 15 Hz admission | remove the separate Drive track cadence authority and consume native-equivalent budget on each eligible Drive invocation | active AMCV track advances state on every eligible object pass; no two scheduler-imposed frozen subticks between updates | do not preserve 15 Hz merely because old docs called it “native frame” |
| Active-track completion can select and retry in the same Process, with fresh integer budget masked | Rust has a useful zero-fresh retry helper but it is outside native owner order and omits pre-budget speed-state work | keep same-invocation continuation and reproduce native pre-budget side effects before residual-only consumption | current track finishes, next track is installed, retry consumes residual only, speed-state write order matches | do not defer continuation one Rust tick; do not add fresh speed twice |
| `g_CurrentFrameCounter` increments once after each normal reached Main_Tick | Rust synthesizes a separate 15 Hz binary frame | make the authoritative gamemd frame basis advance once per completed native-equivalent sim pass; model the late session-end branch as a terminal exit that also skips pacing/remaining tail | object logic observes N; normal tail commits N+1; late session-end exit leaves N and does not run normal post-counter helpers | do not retain the 15 Hz synthetic counter as gamemd's frame counter; do not model the flag as “counter frozen but normal tail continues” |
| Local speed controls inter-tick pacing, not per-call content | Rust directionally changes steps per wall second but may batch multiple steps in one host update; speed 0 is capped at 60 | preserve one native-equivalent content pass per scheduled tick and review app input/state-machine/render interleaving around catch-up batches | stored speeds `1..6` follow bucket-derived target pacing; speed 0 is workload-limited; no extra Drive work inside one tick | do not claim exact runtime FPS without a retail probe |
| Offline Options prevents Main_Tick entry | current offline pause matches | preserve current offline freeze while future network modal mode uses the separate guarded branch | N offline modal service iterations leave sim tick/frame unchanged with responsive UI | do not infer that `g_GameState!=0` means offline Options continues moving |

Recommended implementation order:

1. update the approved design/contract with the verified one-pass/one-Drive-invocation rule and late native frame basis;
2. move ordinary ground Drive authority into the live per-object host without changing unrelated class categories;
3. remove the separate 15 Hz Drive admission gate and preserve same-invocation residual retry;
4. correct the authoritative frame counter independently of render/animation clocks;
5. add exact ordering/gate fixtures before broader locomotion/pathfinding tuning;
6. perform a retail runtime probe only for realized wall cadence and jitter, not to override the static mechanism.

The two deferred questions do not block the static per-object Drive integration for a new local match. They do block claims about exact focus-transition and save-resume parity.

## 12. Stale Docs and Replacement Wording

### `FRAME_BASIS_ONE_INCREMENT_ONE_LOGIC_STEP_GHIDRA_REPORT.md`

Stale idea: `Main_Tick -> LogicClass::AI -> FootClass::Locomotion_AI`, DriveTrack once per MainTick, and a 15 Hz native movement frame.

Replacement wording:

> On each Main_Tick path that reaches the late per-tick live-object function at 0x0055AFB0, the main live vector invokes object vtable +0x5C in forward live order. A normal eligible Unit reaches UnitClass::AI -> FootClass::AI; Foot first runs TechnoClass::AI_Update, including +0xC4 and Mission_Dispatch, then calls the active locomotor interface +0x40 once. For stock Drive this resolves to DriveLocomotionClass::Process @ 0x004B0500. Drive Process has no independent 15 Hz motion gate and can call Process_Drive_Track twice in one invocation; the second call masks fresh speed and consumes residual only. g_CurrentFrameCounter increments later once per normal reached Main_Tick; a late session-end flag instead takes a terminal exit that also skips the pacing wait and remaining normal tail helpers.

### `FRAME_COUNTER_NONADVANCE_PAUSE_SCENARIO_MATRIX_GHIDRA_REPORT.md`

Add this qualifier:

> The `g_GameState != 0` row describes Main_Tick after entry. Standard offline campaign/skirmish Options loops on modal pump 0x00623120, whose modes 0 and 5 do not call Main_Tick; therefore actual offline Options freezes the live-object pass and frame counter.

### July 20 Drive contract and approved design

Replace the open static scheduling blocker with:

> Static native scheduling is resolved: one Drive Process opportunity per eligible Unit/Foot live-object turn, one live-object pass per reached Main_Tick, no separate 15 Hz Drive movement gate, and a late g_CurrentFrameCounter increment per normal reached tick. Default local `GameSpeed=1` supplies a one-bucket 16 ms timer-domain budget. Realized retail wall cadence and jitter remain runtime-unmeasured.

### Current Rust source comments

- `src/sim/world/mod.rs:2172..2181` must not describe a synthetic 15 Hz counter as the native `g_CurrentFrameCounter` contract.
- `src/sim/movement/movement_step.rs:42..69` must not describe the explicit 15 Hz Drive gate as native frame admission.
- `src/sim/world/mod.rs:2213..2232` correctly calls out the future absorption of ground movement into the per-object stage; this report supplies the missing binary proof.

## 13. Negative Facts / Do Not Do

- Do not model normal Drive locomotion as a separate 15 Hz subsystem. No such gate controls the active call chain.
- Do not treat two DriveTrack calls as two native frames. The second is a same-invocation residual retry.
- Do not finish all Unit missions before moving the first Unit. Native mission and locomotion are per-object interleaved.
- Do not increment the gamemd frame basis at 15 Hz independently of `Main_Tick`.
- Do not use `GameSpeed` to multiply per-call Drive content. It controls pacing.
- Do not equate a nominal 16 ms timer budget with measured 62.5 FPS. Runtime measurement is still absent.
- Do not use Rust-vs-Rust hashes or hand-computed cadence goldens as gamemd parity evidence.
- Do not infer offline modal movement from the body-level `g_GameState` branch; caller ownership prevents entry.
- Do not port COM/vtable architecture literally. Preserve the verified Rust-native owner, order, gates, state writes, and retry semantics.
- Do not claim pathfinding parity from this scheduling result. It identifies a load-bearing source of pathing-visible drift; A* and cell-legality mechanisms remain separate evidence domains.

## Sources

Primary binary evidence, all read-only against active retail `gamemd.exe`:

- `get_current_program_info()` — confirmed program name/path, PE x86, image base `0x00400000`.
- `disassemble_function(address="0x0048CCC0")` — outer Main_Game call/back-edge.
- `decompile_function(address="0x0055D360")` and `disassemble_function(address="0x0055D360")` — Main_Tick gates, replay convergence, single per-tick call, late counter, pacing setup.
- `disassemble_function(address="0x0055AFB0")` — main live-vector loop and mutation semantics.
- `read_memory(0x007F5C6C, 0x64)`, `read_memory(0x0080CC68, 0x20)`, `read_memory(0x00842D80, 0x30)` — Unit vtable/COL/type descriptor.
- `disassemble_function(address="0x007360C0")` — Unit normal Foot call.
- `disassemble_function(address="0x004DA530")` — Techno-first order, concrete locomotor gates, single interface call.
- `disassemble_function(address="0x006F9E50")` — `+0xC4` increment and Mission_Dispatch.
- `read_memory(0x007E7EAC, 0x50)`, `read_memory(0x007FFDE8, 0x20)`, `read_memory(0x00820248, 0x40)` — Drive vtable/COL/type descriptor.
- `decompile_function(address="0x004B0500")` and `disassemble_function(address="0x004B0500")` — Drive top-level branches and both DriveTrack call sites.
- `decompile_function(address="0x004B0F20")` and `disassemble_function(address="0x004B0F20")` — speed-state order, fresh-speed mask, residual add.
- `disassemble_function(address="0x0055E160")` and `disassemble_function(address="0x006C8C40")` — local wait and 16 ms timer bucket.
- `decompile_function(address="0x00671EA0")`, `decompile_function(address="0x00697F10")`, `disassemble_function(address="0x0052CD70")`, `decompile_function(address="0x005E7460")`, `get_xrefs_to(address="0x00A8EB60")` — GameSpeed source chain.
- `disassemble_function(address="0x00623120")` and `disassemble_function(address="0x004E1D00")` — offline/network modal ownership.

Direct retail-data evidence:

- `ini/rules.ini:2506`
- `ini/rulesmd.ini:3026`
- `ini/rulesmd.ini:6969..7000`

Existing research reconciled:

- `docs/research/DRIVE_PROCESS_MOVEMENT_TICK_ORDER_GHIDRA_REPORT.md`
- `docs/research/DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md`
- `docs/research/LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`
- `docs/research/TECHNOCLASS_AI_MIGRATION_BOUNDARY_GHIDRA_REPORT.md`
- `docs/research/GAME_SPEED_SETTING_RATE_VS_CONTENT_GHIDRA_REPORT.md`
- `docs/research/LIVE_SKIRMISH_PACING_PATH_GHIDRA_REPORT.md`
- `docs/research/MODAL_PUMP_00623120_SERVICE_TICK_CONTRACT_GHIDRA_REPORT.md`
- `docs/research/FRAME_COUNTER_NONADVANCE_PAUSE_SCENARIO_MATRIX_GHIDRA_REPORT.md`
- `docs/contracts/2026-07-20-ground-drive-process-track-stepping-implementation-contract.md`
- `docs/plans/2026-07-20-per-object-ground-movement-drive-process-design.md`
- `docs/research/traces/AMCV_OPEN_GROUND_DRIVE_RETRACE_20260720.md`

Current Rust source read directly on 2026-07-20:

- `src/app_types.rs`
- `src/app_sim_tick.rs`
- `src/sim/world/mod.rs`
- `src/sim/world/techno_ai.rs`
- `src/sim/movement/drive_locomotion.rs`
- `src/sim/movement/movement_tick.rs`
- `src/sim/movement/movement_step.rs`
- `src/sim/movement/drive_track.rs`
