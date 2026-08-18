# Dock RateTimer Frame-Counter Ordering -- Ghidra Research Report

**Address(es):** `0x0055D360`, `0x004C9220`, `0x004C93D0`, `0x004C9480`, `0x00426630`, `0x00737430`, `0x0073D630`, `0x006F9E50`, `0x005B3060`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** frame-counter ordering for dock radio `0x16`, `DriveLocomotion::Do_Turn`, `RateTimer::Set/Current`, Mission_Deploy_Building unload-start timer-cluster writes, and the same-tick `TechnoClass::AI_Update` timer-cluster consumer.  
**Non-Scope:** exact source/meaning of the `Unit+0x104` non-frame payload, exact `Unit+0x110` initializer/writer set, and all global timer users outside this dock path.  
**Confidence:** High for ordering and same-tick elapsed behavior; Medium for Rust delta because Rust was scanned structurally, not exhaustively tested here.  
**Active in YR:** Yes. The covered path is the stock YR refinery harvester path reached by `UnitClass::Receive_Radio(0x16)` and `UnitClass::Mission_Deploy_Building`.

## 1. Overview

GameMD reads and writes `g_CurrentFrameCounter` during logic, then increments the global counter near the end of `Main_Tick`. For the dock path, this means `RateTimer::Set` and Mission_Deploy_Building unload-start write the current tick's pre-increment frame. Any same-tick `Current`/remaining read sees elapsed `0`; the first elapsed frame is only visible after the next `Main_Tick` begins.

This matters for Plan C because the dock facing gate and unload accumulator are frame-stamped state machines, not per-call decrement counters. A Rust implementation must preserve the `start_frame=N, same-tick elapsed=0, next-frame elapsed=1` contract.

## 2. Class Layout / Key Offsets

| Owner | Offset | Meaning in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| global | `0x00A8ED84` | `g_CurrentFrameCounter` | `Main_Tick @ 0x0055DE73..0x0055DE81`; timer reads in `RateTimer` and `TechnoClass::AI_Update` | Yes |
| `RateTimer` / `FacingClass` | `+0x00` | target/current packed 32-bit value; low 16 bits are the animated value | `RateTimer::Set @ 0x004C92BD`; `RateTimer::Current @ 0x004C945E` | Yes |
| `RateTimer` / `FacingClass` | `+0x04` | source/previous packed value used for interpolation | `RateTimer::Set @ 0x004C92B1` | Yes |
| `RateTimer` / `FacingClass` | `+0x08` | start frame | `RateTimer::Set @ 0x004C92E3`; `RateTimer::Current @ 0x004C93DD..0x004C93F6` | Yes |
| `RateTimer` / `FacingClass` | `+0x10` | duration in frames | `RateTimer::Set @ 0x004C92E8`; `RateTimer::Current @ 0x004C93E0` | Yes |
| `RateTimer` / `FacingClass` | `+0x14` | rate/step; <= 0 snaps to target | `RateTimer::Set @ 0x004C9235..0x004C923E`; `RateTimer::Current @ 0x004C93D2..0x004C93DB` | Yes |
| `UnitClass` | `+0x388` | primary/body facing `RateTimer`; dock `0x16` drives this to `0x4000` | `DriveLocomotion::Do_Turn @ 0x004B0F01..0x004B0F0A` | Yes |
| `UnitClass` | `+0xF8` | unload accumulator/cadence counter tested against `HarvesterDumpRate * 900` | `Mission_Deploy_Building @ 0x0073E35B..0x0073E374`; `TechnoClass::AI_Update @ 0x006FABF1..0x006FAC06` | Yes |
| `UnitClass` | `+0x100` | unload-cluster start frame | Mission write `0x0073DFF3`; AI update rewrite `0x006FAC16` | Yes |
| `UnitClass` | `+0x104` | unload-cluster secondary payload copied with the timer restart | Mission write `0x0073DFF5..0x0073DFF9`; AI update rewrite `0x006FAC1C` | Yes, meaning deferred |
| `UnitClass` | `+0x108` | unload-cluster duration; Mission_Deploy starts it as `1` | Mission write `0x0073DFFC`; AI update rewrite `0x006FAC22` | Yes |
| `UnitClass` | `+0x10C` | recurring unload-cluster duration/period; Mission_Deploy sets `1` | Mission write `0x0073DFED`; AI update read `0x006FABE7..0x006FABEF` | Yes |
| `UnitClass` | `+0x110` | amount added to `+0xF8` when the unload-cluster timer expires | AI update read/add `0x006FABF1..0x006FAC06` | Yes, initializer deferred |

## 3. Core Logic

### 3.1 `Main_Tick` increments the frame after logic

`Main_Tick @ 0x0055D360` executes gameplay logic before the global frame increment:

1. `GScreenClass__Input`, `LogicClass__AI`, optional `House_AI_Tick`, `Map__Logic`, and `RenderFrame_main` run at `0x0055D897..0x0055D8F2`.
2. `LogicClassPerTickUpdateLiveVector` runs at `0x0055DC99`.
3. Late maintenance/network/render work follows.
4. Only then, if pause/network gates allow it, the frame counter increments:
   - `0x0055DE73`: load `g_CurrentFrameCounter`
   - `0x0055DE7E`: `INC EDX`
   - `0x0055DE81`: store incremented value back to `0x00A8ED84`

**Same-tick consequence:** all timer writes during object AI in that tick store frame `N`; later reads in that same tick also see frame `N`. The first tick that can see elapsed `1` is the next logic tick, after `Main_Tick` has completed the increment.

### 3.2 `RateTimer::Set` writes the current pre-increment frame

`DriveLocomotionClass::Do_Turn @ 0x004B0EF0` is a thin wrapper:

- `0x004B0F01`: loads owner from locomotor `+0x08`
- `0x004B0F04`: adds `0x388`
- `0x004B0F0A`: calls `RateTimer::Set`

`RateTimer::Set @ 0x004C9220`:

1. If target low word already equals new low word, it returns false without rewriting fields (`0x004C922A..0x004C922D`, `0x004C92F7`).
2. If rate is positive, it snapshots the current interpolated value into `+0x04` before retargeting (`0x004C9240..0x004C92B1`).
3. It writes the new target to `+0x00` at `0x004C92BD`.
4. It writes `g_CurrentFrameCounter` to `+0x08` at `0x004C92C4..0x004C92E3`.
5. It writes duration `abs(target_low - source_low) / rate` to `+0x10` at `0x004C92CA..0x004C92E8`.

**Same-tick consequence:** a `RateTimer::Current` after this `Set` in the same `Main_Tick` sees `elapsed = g_CurrentFrameCounter - start_frame = N - N = 0`.

### 3.3 `RateTimer::Current` and CDTimer helpers use `< duration`, not `<=`

`RateTimer::Current @ 0x004C93D0`:

- Reads start `+0x08`, duration `+0x10`, rate `+0x14`.
- If start is not `-1`, computes `elapsed = g_CurrentFrameCounter - start_frame`.
- If `elapsed >= duration`, it returns the target (`0x004C93EE..0x004C93F4` branches to `0x004C945A`).
- Otherwise it uses `remaining = duration - elapsed`.

`CDTimerClass__GetTimeRemaining @ 0x00426630` has the same boundary:

- If `elapsed < duration`, return `duration - elapsed`.
- Otherwise return `0`.

`CDTimerClass__Remaining @ 0x004C9480` returns boolean remaining and also treats `elapsed >= duration` as expired.

**Duration-one consequence:** a timer started with `start=N, duration=1` has remaining `1` in the same tick and remaining `0` once the global frame reaches `N+1`.

### 3.4 Dock `0x16` and Mission_Deploy facing gate both use elapsed-zero semantics

`UnitClass::Receive_Radio(0x16) @ 0x00737430`:

- Calls `RateTimer::Current(Unit+0x388)` at `0x007376C9..0x007376D4`.
- If low word is not `0x4000`, calls active locomotor vtable `+0x4C(0x4000)` at `0x007376E0..0x00737709`.

`UnitClass::Mission_Deploy_Building @ 0x0073D630`:

- Path gate succeeds before facing gate (`0x0073DEE0..0x0073DEE9`).
- Calls `RateTimer::Current(Unit+0x388)` at `0x0073DF56..0x0073DF61`.
- Accepts the facing window if `((current >> 7) + 1) & 0x1FE == 0x80` (`0x0073DF66..0x0073DF78`).
- If not accepted and `Unit+0x6AF == 0`, calls locomotor `+0x4C(0x4000)` and returns `5` (`0x0073DF7A..0x0073DFB8`).

**Ordering consequence:** if Mission_Deploy calls `+0x4C(0x4000)`, then returns `5`, any later same-tick read of that same `RateTimer` sees elapsed zero because `RateTimer::Set` used the unchanged current frame. It must not be treated as having advanced one frame immediately.

### 3.5 Mission_Deploy unload-start writes the cluster before the same AI_Update consumer

When the path and facing gates accept and `Unit+0x6D1 == 0`, Mission_Deploy initializes unload state:

- `0x0073DFD0`: `Unit+0xF8 = 0`
- `0x0073DFDA`: `Unit+0x6D1 = 1`
- `0x0073DFE0`: read `g_CurrentFrameCounter`
- `0x0073DFED`: `Unit+0x10C = 1`
- `0x0073DFF3`: `Unit+0x100 = g_CurrentFrameCounter`
- `0x0073DFF9`: `Unit+0x104 = stack payload`
- `0x0073DFFC`: `Unit+0x108 = 1`
- `0x0073E093`: `Unit+0xBC = 3`
- `0x0073E09D`: fall through to the mission timer epilogue at `0x0073E289`

`TechnoClass::AI_Update @ 0x006F9E50` calls `MissionClass::Mission_Dispatch` at `0x006FA655`, then later handles the `+0xF8..+0x110` cluster at `0x006FABC4..0x006FAC28`.

The cluster consumer:

1. Skips buildings (`What_Am_I == 6`) at `0x006FABBA..0x006FABC2`.
2. Reads `+0x100` and `+0x108` at `0x006FABC4..0x006FABCA`.
3. If start is not `-1`, computes `elapsed = g_CurrentFrameCounter - +0x100` and expires only when `elapsed >= +0x108` (`0x006FABD0..0x006FABDF`).
4. If not expired, falls through to clear `+0xFC = 0` at `0x006FAC2A`.
5. If expired and `+0x10C != 0`, sets `+0xFC = 1`, adds `+0x110` to `+0xF8`, then restarts the cluster with `+0x100 = g_CurrentFrameCounter`, `+0x104 = stack payload`, `+0x108 = +0x10C` (`0x006FABE7..0x006FAC22`).

**Same-AI_Update consequence:** Mission_Deploy can write `+0x100=N, +0x108=1` inside `Mission_Dispatch`; the later cluster consumer in the same `TechnoClass::AI_Update` still sees `elapsed=0`, so it does not increment `+0xF8` on the unload-start tick. The first possible `+0xF8 += +0x110` is the next logic tick at frame `N+1`.

### 3.6 Mission return-delay storage also stamps the pre-increment frame

`MissionClass::Mission_Dispatch @ 0x005B3060` is called from `TechnoClass::AI_Update` at `0x006FA655`. For mission `0x10`, it dispatches vtable `+0x23C` at `0x005B3260..0x005B326A`, then writes:

- `Mission+0xC8 = g_CurrentFrameCounter`
- `Mission+0xCC = stack payload`
- `Mission+0xD0 = mission_return_delay`

The same function uses `elapsed = g_CurrentFrameCounter - Mission+0xC8` and only dispatches when elapsed is not less than the stored delay (`0x005B308C..0x005B30A7`). Direct return `5` from the facing-not-ready branch therefore schedules the next Mission_Deploy pass after five frame-counter increments, not after five immediate AI calls.

## 4. INI Keys

No INI key is newly decoded by this report. Relevant values are consumed by already-identified paths:

| INI key | Current role in this slice | Evidence | Active in YR |
|---|---|---|---|
| `ROT=` on `CMIN`/`HARV` | determines `RateTimer+0x14` through locomotor/facing setup outside this slice | prior RateTimer/Drive reports; current slice verifies consumers of the rate field | Yes |
| `HarvesterDumpRate=` | Mission_Deploy compares `Unit+0xF8` against `HarvesterDumpRate * 900` | `0x0073E35B..0x0073E374` | Yes |

## 5. Integration Points

| Point | Timing role | Evidence | Active in YR |
|---|---|---|---|
| `Main_Tick` | global frame increments after logic/object updates | `0x0055D897..0x0055DE81` | Yes |
| `UnitClass::Receive_Radio(0x16)` | starts or checks the `Unit+0x388` facing timer | `0x007376C9..0x00737709` | Yes |
| `DriveLocomotionClass::Do_Turn` | maps locomotor `+0x4C` to `RateTimer::Set(owner+0x388)` | `0x004B0EF0..0x004B0F0F` | Yes |
| `UnitClass::Mission_Deploy_Building` | checks path/facing, starts unload cluster, returns mission delay | `0x0073DEE0..0x0073E2BE` | Yes |
| `MissionClass::Mission_Dispatch` | stores returned mission delay with the current frame | `0x005B3260..0x005B327A`; general timer gate `0x005B308C..0x005B30A7` | Yes |
| `TechnoClass::AI_Update` | calls Mission_Dispatch before the unload-cluster consumer | `0x006FA655`; `0x006FABC4..0x006FAC28` | Yes |

## 6. Current Rust Implementation Status

Rust currently advances `Simulation::binary_frame` at the start of `Simulation::advance_tick`:

- `src/sim/world/mod.rs:1101..1114`: `total_sim_ms` increments, then `binary_frame` is derived before the rest of tick logic.

Rust's `FacingClass` model is structurally close to GameMD's `RateTimer` semantics:

- `src/sim/movement/facing_class.rs`: `set` snapshots current value before writing a new target, stores `start_frame`, and `current(frame)` uses `elapsed = frame - start`.

The stock refinery dock sequence still uses a dock-local approximation:

- `src/sim/miner/miner_dock_sequence.rs:778..805`: `sync_dock_facing` calls `FacingClass::set` and writes `entity.facing` from the animated value.
- `src/sim/miner/miner_dock_sequence.rs:805..836`: `start_unload_deploy` directly writes `entity.facing = DOCK_FACING_EAST`, emits `DockDeploy`, and seeds `unload_timer = unload_tick_interval - 10`.
- `src/sim/miner/miner_dock_sequence.rs:854..934`: unload cadence is modeled as a signed countdown, not the `+0xF8..+0x110` frame-stamped cluster.

Rust delta for this slot:

- Same-tick elapsed can be preserved if all dock `Set`/`Current`/cluster reads use one stable per-tick frame value.
- Absolute frame values are currently one tick earlier/later relative to GameMD's end-of-tick increment depending on observation point. For dock-local difference timers this does not require a local subtract-one hack; global frame parity remains a separate timing-model concern.
- Plan C should not implement unload-start as "seed countdown and immediately decrement in the same tick." GameMD writes `+0x100=N,+0x108=1` and the later same-AI_Update consumer sees elapsed zero.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Main_Tick` frame increment order | verified | decompile `0x0055D360`; assembly `0x0055D897..0x0055DE81` | none for this slice |
| `RateTimer::Set` start-frame write | verified | decompile/disassembly `0x004C9220..0x004C92FC` | none |
| `RateTimer::Current` elapsed/expiry boundary | verified | decompile/disassembly `0x004C93D0..0x004C9466` | none |
| `CDTimerClass__GetTimeRemaining` boundary | verified | decompile `0x00426630` | none |
| `CDTimerClass__Remaining` boolean boundary | verified | decompile `0x004C9480` | none |
| `DriveLocomotion::Do_Turn` to `Unit+0x388` | verified | decompile/disassembly `0x004B0EF0..0x004B0F0F` | none |
| `UnitClass::Receive_Radio(0x16)` facing sync | verified | decompile/disassembly `0x00737430`, key branch `0x007376C9..0x00737709` | none for timing |
| `Mission_Deploy_Building` path/facing/unload-start ordering | verified | decompile/disassembly `0x0073DEE0..0x0073E09D` | exact `+0x104` payload meaning out-of-scope |
| `MissionClass::Mission_Dispatch` return-delay storage | verified | decompile/disassembly `0x005B3060`, key branch `0x005B3260..0x005B327A` | none for timing |
| `TechnoClass::AI_Update` cluster after mission dispatch | verified | decompile/disassembly `0x006FA655`, `0x006FABC4..0x006FAC28` | exact `+0x110` initializer out-of-scope |
| Rust `binary_frame` tick placement | verified | `src/sim/world/mod.rs:1101..1114` | full global timing-model refactor out-of-scope |
| Rust dock countdown approximation | touched-not-exhausted | `src/sim/miner/miner_dock_sequence.rs:778..934` | implementation belongs to Plan C |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-01 -- Does Main_Tick increment the global frame before or after gameplay logic? -> After logic/object work, at the late gate near function end.` (evidence: `0x0055D897..0x0055DE81`)
- `[RESOLVED] OQ-02 -- Does RateTimer::Set use the current pre-increment frame? -> Yes; it writes `g_CurrentFrameCounter` to `+0x08` after retargeting.` (evidence: `0x004C92C4..0x004C92E3`)
- `[RESOLVED] OQ-03 -- Can same-tick RateTimer::Current see elapsed 1 after Set? -> No; same-tick Set/Current both read the same global frame, so elapsed is 0.` (evidence: `0x004C92E3`, `0x004C93DD..0x004C93F6`, `0x0055DE73..0x0055DE81`)
- `[RESOLVED] OQ-04 -- Is duration expiry inclusive or exclusive? -> Expired when `elapsed >= duration`; active only when `elapsed < duration`.` (evidence: `0x004C93EE..0x004C93F4`, `0x00426630`, `0x004C9480`)
- `[RESOLVED] OQ-05 -- Does dock radio 0x16 call the active Drive locomotor timer, not write body facing? -> Yes; it calls locomotor vtable `+0x4C(0x4000)` when Current low word is not `0x4000`.` (evidence: `0x007376C9..0x00737709`, `0x004B0EF0..0x004B0F0F`)
- `[RESOLVED] OQ-06 -- Does Mission_Deploy use the same RateTimer elapsed behavior for its facing gate? -> Yes; it calls `RateTimer::Current(Unit+0x388)`, tests the quantized East window, and otherwise calls `+0x4C(0x4000)` and returns `5`.` (evidence: `0x0073DF56..0x0073DFB8`)
- `[RESOLVED] OQ-07 -- Does unload-start write the cluster before AI_Update's cluster consumer can run? -> Yes; Mission_Dispatch is called at `0x006FA655`; Mission_Deploy writes the cluster; AI_Update's consumer is later at `0x006FABC4..0x006FAC28`.` (evidence: `0x006FA655`, `0x0073DFF3..0x0073DFFC`, `0x006FABC4..0x006FAC28`)
- `[RESOLVED] OQ-08 -- Can the unload-start tick immediately increment `+0xF8` through the same AI_Update cluster consumer? -> No; `+0x100=N,+0x108=1` yields elapsed 0 in that same AI_Update, so the not-expired branch runs.` (evidence: `0x0073DFF3..0x0073DFFC`, `0x006FABD0..0x006FABE5`)
- `[RESOLVED] OQ-09 -- When is the first possible unload-cluster increment after unload start? -> The next logic tick/frame, when `g_CurrentFrameCounter == N+1`, because duration 1 expires at elapsed 1.` (evidence: `0x00426630`; `0x006FABD0..0x006FABE7`; `0x0055DE73..0x0055DE81`)
- `[RESOLVED] OQ-10 -- Does Mission_Dispatch also stamp return delays with the same frame model? -> Yes; after mission vtable return it writes `Mission+0xC8 = g_CurrentFrameCounter` and `Mission+0xD0 = return delay`.` (evidence: `0x005B3260..0x005B327A`)
- `[DEFERRED] OQ-11 -- What exact value/source is copied into `Unit+0x104`?` (category: out-of-scope; reason: slot 5 only resolves frame-counter ordering; next-step-if-pursued: slot 2 `Unit+0x104` dataflow around `0x0073DFF5` and `0x006FAC1C`)
- `[DEFERRED] OQ-12 -- Where is `Unit+0x110` initialized for stock unload?` (category: out-of-scope; reason: slot 5 only proves when `+0x110` is consumed; next-step-if-pursued: slot 1 writer/default audit)
- `[DEFERRED] OQ-13 -- Should Rust globally move `binary_frame` update to the end of `advance_tick`?` (category: requires-different-system-context; reason: this slice proves the dock consequence but global frame update affects combat, animation, timers, and tests; next-step-if-pursued: global timing-model design/audit)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `RateTimer::Set` stores the current frame; same-tick `Current` sees elapsed 0. | `0x004C92E3`; `0x004C93DD..0x004C93F6`; `0x0055DE73..0x0055DE81` | partial; `FacingClass` is close, dock uses it locally | `src/sim/movement/facing_class.rs`; future dock RateTimer bridge | Preserve one stable per-tick frame for `set/current`; no immediate one-frame advancement after `Do_Turn`. | `rate_timer_set_then_current_same_tick_sees_elapsed_zero` | Do not subtract or add 1 inside `RateTimer::Current` to "fix" same-tick behavior. |
| Mission_Deploy unload-start writes `+0x100=N,+0x108=1,+0x10C=1` and the same AI_Update cluster pass must not increment `+0xF8`. | `0x0073DFF3..0x0073DFFC`; `0x006FA655`; `0x006FABD0..0x006FAC06` | missing; Rust seeds `unload_timer` countdown in `start_unload_deploy` | `src/sim/miner/miner_dock_sequence.rs`; possible `sim` timer-cluster state | First unload-start tick should initialize unload-active state but not drain or increment the unload accumulator. | `unload_cluster_does_not_increment_on_unload_start_tick` | Do not seed countdown as `interval - 10` if that causes same-tick progress equivalent to elapsed 1. |
| Duration `1` expires on the next frame, not the same frame. | `0x00426630`; `0x006FABD0..0x006FABE7` | missing for unload cluster; present conceptually in `FacingClass` | future unload timer-cluster model | On the next binary frame after unload start, the cluster may add `+0x110` to `+0xF8` and restart `+0x100` to the new frame. | `unload_cluster_increments_once_on_next_binary_frame` | Do not model `+0x108` as a self-decrementing counter. It is a duration compared against a frame stamp. |
| Mission return delay is stamped with the same current frame after the vtable mission returns. | `0x005B3260..0x005B327A` | unchecked; Rust miner dock phases bypass generic mission dispatch | future mission timer bridge, if Plan C models mission timers | Direct return `5` from facing-not-ready delays the next Mission_Deploy pass until five frame-counter increments have elapsed. | `mission_deploy_return_five_reschedules_after_five_frames` | Do not poll Mission_Deploy every Rust tick while the binary would be waiting on the mission timer. |
| Rust `binary_frame` currently updates before tick logic, while GameMD increments after logic. | GameMD `0x0055DE73..0x0055DE81`; Rust `src/sim/world/mod.rs:1101..1114` | mismatch in absolute frame observation, but dock-local differences can still match if set/read use the same frame value | `src/sim/world/mod.rs`; all frame-driven systems | For Plan C, use consistent per-tick frame stamps. A global end-of-tick frame refactor needs separate audit. | `dock_timers_use_stable_logic_frame_with_no_same_tick_elapsed` | Do not apply a dock-local `binary_frame - 1` hack unless the global timing service is designed around it. |

## Negative Facts / Do Not Do

- Do not treat `RateTimer::Set` followed by same-tick `Current` as elapsed `1`; GameMD sees elapsed `0`.
- Do not self-decrement `RateTimer` or the unload cluster each call. They are passive frame-stamp/duration systems.
- Do not start `+0xF8` progression on the same tick Mission_Deploy initializes unload state. Same AI_Update reads duration `1` as still remaining.
- Do not fix dock timing by blindly subtracting one from `sim.binary_frame`; that can preserve this path while breaking other direct `frame % N` consumers.
- Do not infer the source/meaning of `Unit+0x104` or the initializer of `Unit+0x110` from this report. This slot only proves ordering and consumption.

## Stale Docs / Follow-up Wording

Replace any wording like:

> GameMD increments `g_CurrentFrameCounter` before object AI, so timers set during a tick can read as one frame elapsed later in that same tick.

with:

> GameMD increments `g_CurrentFrameCounter` near the end of `Main_Tick`, after gameplay/object logic. Timers set during object AI store the pre-increment frame; same-tick reads see elapsed `0`, and elapsed `1` appears on the next logic tick.

Replace any dock-unload wording like:

> Mission_Deploy starts unload and the unload timer immediately advances on that tick.

with:

> Mission_Deploy starts unload by writing `+0x100 = g_CurrentFrameCounter`, `+0x108 = 1`, and `+0x10C = 1`. The later same-`TechnoClass::AI_Update` cluster consumer sees elapsed `0`, so `+0xF8` does not increment until the next frame.

## Sources

- Ghidra live read-only decompile/disassembly:
  - `Main_Tick @ 0x0055D360`
  - `RateTimer::Set @ 0x004C9220`
  - `RateTimer::Current @ 0x004C93D0`
  - `CDTimerClass__Remaining @ 0x004C9480`
  - `CDTimerClass__GetTimeRemaining @ 0x00426630`
  - `DriveLocomotionClass::Do_Turn @ 0x004B0EF0`
  - `UnitClass::Receive_Radio @ 0x00737430`
  - `UnitClass::Mission_Deploy_Building @ 0x0073D630`
  - `TechnoClass::AI_Update @ 0x006F9E50`
  - `MissionClass::Mission_Dispatch @ 0x005B3060`
- Rust scan:
  - `src/sim/world/mod.rs`
  - `src/sim/movement/facing_class.rs`
  - `src/sim/miner/miner_dock_sequence.rs`
- Prior research orientation only:
  - `docs/research/RATETIMER_CURRENT_FRAME_COUNTER_HELPERS_GHIDRA_REPORT.md`
  - `docs/research/GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`
  - `docs/research/DRIVELOCOMOTION_VTABLE_0X4C_TIMING_SYNC_METHOD_GHIDRA_REPORT.md`
  - `docs/research/miner/DOCK_0X16_DOTURN_RATETIMER_UNLOAD_GATE_RECHECK_20260526.md`

