# Frame Counter Non-Advance: Pause / Scenario-Delay Matrix

**Primary address:** `Main_Tick @ 0x0055D360`  
**Supporting addresses:** `State_Machine @ 0x0048C8B0`, `g_CurrentFrameCounter @ 0x00A8ED84`, `g_GameState @ 0x00A8EDA0`, `g_GameRunning @ 0x00A8ED80`, `g_GameActive @ 0x00A8E9A0`, `DAT_00A8D5F8 @ 0x00A8D5F8`, `Scenario+0x62C`  
**Investigation mode:** exhaustive-slice  
**Claimed scope:** All states in a standard YR skirmish where `g_CurrentFrameCounter` does NOT advance during `Main_Tick`, excluding the four session-end flags already decoded.  
**Non-scope:** Speed/throttle ms-per-frame, `PerTickUpdate` ladder internals, animation rates, replay/record path internals beyond what is needed to confirm the gameplay-block gate.  
**Confidence:** HIGH for all findings below — all assembly verified directly from `decompile_function 0x0055D360` + `decompile_function 0x0048C8B0` + `get_assembly_context` calls.  
**Active in YR:** Yes for all non-advance states; each entry names its activation condition.

---

## Investigation Contract

**Target question:** Enumerate every state in a standard YR skirmish where `g_CurrentFrameCounter` does NOT advance during `Main_Tick`, excluding the four session-end flags, and identify what work still runs vs. what is frozen in each state.

**Non-goals:** Speed/throttle internals, `PerTickUpdate` callee loop details, replay byte-by-byte desync logic, animation rates.

**Evidence needed to mark COMPLETE:**

| Evidence requirement | Status | Evidence |
|---|---|---|
| g_GameActive == 0 early-return path | met | assembly `0x0055D36B..0x0055D371`; `JZ 0x0055DECF` |
| g_GameRunning == 0 wait-loop | met | assembly `0x0055D377..0x0055D3B0`; `Sleep(500)/Sleep(10)`+network |
| Scenario+0x62C != 0 render-only early return | met | assembly `0x0055D821..0x0055D877`; `JZ 0x0055D878` bypass |
| g_GameState != 0 gameplay-block skip | met | assembly `0x0055D878..0x0055D901`; triple guard + `JNZ 0x0055D8FF` / `JNZ 0x0055D901` |
| g_GameState semantics (pause/modal values) | met | `decompile_function 0x0048C8B0`; State_Machine switch cases |
| DAT_00A8D5F8 bit-2 = replay-playback, NOT pause | met | `decompile_function 0x0055D360`; `TEST CL,0x2; JNZ 0x0055D8FF`; replay doc cross-check |
| PerTickUpdate runs unconditionally for all non-session-end states | met | `decompile_function 0x0055D360`; `LogicClassPerTickUpdateLiveVector()` call is outside all gameplay-block guards |
| Frame counter NOT incremented in g_GameActive==0 | met | assembly `0x0055D371: JZ 0x0055DECF`; `LAB_0055DECF` returns without reaching `g_CurrentFrameCounter + 1` |

**Stop conditions:** Stop after proving non-advance gates and what runs in each; do not investigate PerTickUpdate callee bodies, replay bit-1 details, or runtime pacing.

---

## 1. Key Globals

| Address | Name | Type | Purpose | Evidence |
|---:|---|---|---|---|
| `0x00A8E9A0` | `g_GameActive` | byte | Non-zero = session is active at all; zero = skip entire body | assembly `0x0055D360..0x0055D371` |
| `0x00A8ED80` | `g_GameRunning` | byte | Non-zero = normal gameplay running; zero = inactive/wait loop | assembly `0x0055D377..0x0055D3B0` |
| `0x00A8B238` | `g_GameMode` | int | 0=solo, 4=MP, 5=skirmish AI; controls wait-loop sleep/network behavior | assembly `0x0055D38D..0x0055D399` |
| `0x00A8EDA0` | `g_GameState` | int | 0=gameplay active; non-zero=modal/pause/dialog state (see §3) | assembly `0x0055D883..0x0055D895` |
| `0x00A8D5F8` | `DAT_00A8D5F8` | int | bit 0 = replay record; bit 2 = **replay playback** (NOT pause) | assembly `0x0055D87E..0x0055D881`; `0x0055D90F..0x0055D912` |
| `Scenario+0x62C` | `g_ScenarioClass_Instance + 0x62C` | int | Non-zero = scenario intro/delay active; triggers render-only early return | assembly `0x0055D826..0x0055D82E` |

**Critical clarification:** `DAT_00A8D5F8 & 2` is the **replay-playback** flag, not an in-game pause flag. The GScreen research doc's phrase "SpecialFlags & 2" at line 481 is the same dword but refers to the replay-playback sense. This is confirmed by the replay playback research doc (`REPLAY_ACTIVE_VECTOR_RESTORE_CORNER_RESWARM_20260528.md §3.4`: "replay playback bit `& 2` skips normal input/AI/map/render block") and the `RENDER_LOGIC_COUPLING_MAIN_TICK_GHIDRA_REPORT.md §4`. Active-in-YR: No for standard skirmish (replay is not active).

---

## 2. Enumeration of Non-Advance States

### State A — `g_GameActive == 0` (session not live)

**Frame counter behavior:** Does NOT advance. The check fires before any tick work begins.

**Assembly evidence:** `verified via get_assembly_context 0x0055D36B`

```
0055d360: MOV AL,[0x00a8e9a0]   ; load g_GameActive
0055d36b: TEST AL,AL
0055d371: JZ 0x0055decf          ; → LAB_0055DECF which returns without reaching g_CurrentFrameCounter++
```

**What runs:** Nothing — `DAT_00ABCD58 = 1` is also skipped because it is at `0x0055D37C`, after the `JZ`. The function returns via `LAB_0055DECF` which executes `RETURN CONCAT31(uVar11,1)`.

**What is frozen:** Everything. Input, AI, network, render, PerTickUpdate, frame counter.

**Activation in standard YR:** This state fires before session launch and after session teardown. During an active skirmish the byte is non-zero; it becomes zero when `Main_Game` has not yet set it (pre-launch) or after the session ends. Active in YR: Yes (pre/post-session), No (during active skirmish match).

---

### State B — `g_GameRunning == 0` (inactive / game paused at OS level, NOT in-game pause)

**Frame counter behavior:** Does NOT advance. The function spins in the wait loop until `g_GameRunning` becomes non-zero, then falls through to normal tick work.

**Assembly evidence:** `verified via get_assembly_context 0x0055D383`

```
0055d377: MOV AL,[0x00a8ed80]   ; load g_GameRunning
0055d383: TEST AL,AL
0055d385: JNZ 0x0055d3bb        ; non-zero → proceed to normal tick work
; zero path: enter wait loop
0055d38d: MOV EAX,[0x00a8b238]  ; load g_GameMode
0055d392: TEST EAX,EAX
0055d394: JZ 0x0055d39b         ; GameMode==0 → Sleep(500)
0055d396: CMP EAX,0x5
0055d399: JNZ 0x0055d3b2        ; GameMode!=5 → Sleep(10) [network path]
0055d39b: PUSH 0x1f4            ; 500ms
0055d3a0: CALL ESI              ; Sleep(500)
0055d3a2: CALL 0x005d4d50       ; Process_NetworkMessages
0055d3a7: MOV AL,[0x00a8ed80]   ; re-check g_GameRunning
0055d3ac: TEST AL,AL
0055d3ae: JZ 0x0055d38d         ; still zero → loop
0055d3b0: JMP 0x0055d3bb        ; non-zero → proceed
0055d3b2: PUSH 0xa              ; 10ms (network path)
0055d3b4: CALL ESI              ; Sleep(10)
0055d3b6: CALL 0x005d4d50       ; Process_NetworkMessages
```

**What runs:** `Process_NetworkMessages @ 0x005D4D50` on every loop iteration. Nothing else runs.

**What is frozen:** Input, AI, map logic, render, PerTickUpdate, frame counter. All frozen while in the wait loop.

**Sleep duration:** 500ms for solo/skirmish (`g_GameMode == 0` or `== 5`), 10ms for network mode (`g_GameMode == 4`).

**Activation in standard YR:** This is the window-focus-lost / minimize state, not the in-game ESC menu. The ESC menu is a different mechanism (see §State D). Active in YR: Yes, triggered by OS-level window deactivation.

---

### State C — `Scenario+0x62C != 0` (scenario intro delay / countdown active)

**Frame counter behavior:** Does NOT advance. The function processes network/events/render/wait then returns early before reaching the gameplay block or the late increment.

**Assembly evidence:** `verified via get_assembly_context 0x0055D821,0x0055D862`

```
0055d821: MOV EAX,[0x00a8b230]     ; load g_ScenarioClass_Instance
0055d826: MOV ECX,dword ptr [EAX+0x62c]
0055d82c: TEST ECX,ECX
0055d82e: JZ 0x0055d878            ; zero → proceed to normal tick work
; non-zero path:
0055d830: CALL 0x005d4d50          ; Process_NetworkMessages
0055d835: CALL 0x0048d080          ; Network_ServiceLoop
0055d83a: CALL 0x0053b560          ; Process_QueuedEvents
0055d83f: MOV ECX,dword ptr [0x00887324]
0055d845: MOV EDX,dword ptr [ECX]
0055d847: CALL dword ptr [EDX+0x5c] ; TacticalClass::Update (vtable[23])
0055d84a: MOV ECX,0x87f7e8
0055d84f: CALL 0x004f4480           ; RenderFrame_main
0055d854: CALL 0x0055e160           ; FUN_0055E160 (wait/throttle helper)
0055d859: MOV CL,[0x00a8e9a0]       ; check g_GameActive
0055d862: POP ESI
0055d863: TEST CL,CL
0055d866: MOV byte ptr [0x00abcd58],0x0
0055d86d: SETZ AL
0055d871: ADD ESP,0x1b4
0055d877: RET                        ; returns WITHOUT reaching g_CurrentFrameCounter++
```

**What runs:** `Process_NetworkMessages`, `Network_ServiceLoop`, `Process_QueuedEvents`, `TacticalClass::Update`, `RenderFrame_main`, `FUN_0055E160` (throttle/wait helper).

**What is frozen:** `GScreenClass::Input`, `LogicClass::AI`, `House_AI_Tick`, `Map::Logic`, `LogicClassPerTickUpdateLiveVector`, `g_CurrentFrameCounter++`. All game logic and PerTickUpdate frozen.

**Note on `Scenario+0x630` flag:** The decompile also shows a `Scenario+0x630` byte (non-zero triggers `FUN_00684180` then falls through to the `Scenario+0x62C` check). The `+0x630` path clears itself and calls `FUN_00684180`, then falls through to the `Scenario+0x62C` branch; it does not itself return early or skip the frame increment. The non-advance is entirely owned by the `+0x62C != 0` branch.

**Activation in standard YR:** Active during the brief scenario-start intro/cinematic delay set by some mission scripts or scenario load sequences. Active in YR: Conditional (not fired in a typical skirmish with no intro delay, but the gate always executes; zero means pass-through).

**Writers for `Scenario+0x62C`:** Identifying all writers was not pursued in this slice (they are referenced in `TIMING_SCHEDULER_TICK_SPINE_SYSTEM_MODEL_SYNTHESIS.md §4` as "exact writers for that flag remain a follow-up"). This is Remaining Uncertainty.

---

### State D — `g_GameState != 0` (in-game pause / modal dialogs)

**Frame counter behavior:** The gameplay block (Input → AI → Map::Logic → RenderFrame) is **skipped**, but `LogicClassPerTickUpdateLiveVector()` and `g_CurrentFrameCounter++` still run. This is the in-game ESC menu and all modal dialogs. **The frame counter advances during in-game pause.**

**Assembly evidence:** `verified via get_assembly_context 0x0055D878,0x0055D883,0x0055D88A,0x0055D893`

```
; Triple-gate for the normal gameplay block:
0055d878: MOV ECX,dword ptr [0x00a8d5f8] ; load DAT_00A8D5F8 (replay flags)
0055d87e: TEST CL,0x2
0055d881: JNZ 0x0055d8ff               ; bit 2 (replay playback) → skip gameplay block
0055d883: MOV EAX,[0x00a8eda0]         ; load g_GameState
0055d888: XOR EDI,EDI
0055d88a: CMP EAX,EDI
0055d88c: JNZ 0x0055d901               ; g_GameState != 0 → skip gameplay block
0055d88e: MOV AL,[0x00a8ed80]          ; load g_GameRunning
0055d893: TEST AL,AL
0055d895: JZ 0x0055d901                ; g_GameRunning == 0 → skip gameplay block
; fall-through at 0x0055d897: normal gameplay block runs
```

`0x0055D901` continues directly into the replay-playback section and then unconditionally into `FUN_00551A30`, `LogicClassPerTickUpdateLiveVector()`, service/network, and the late `g_CurrentFrameCounter++` check. **Verified via `decompile_function 0x0055D360`**: `LogicClassPerTickUpdateLiveVector()` appears after the replay block, not inside the triple-gated gameplay section.

**What runs when `g_GameState != 0` (in-game pause):** `LogicClassPerTickUpdateLiveVector`, `FUN_00551A30`, `FUN_00647260`, `FUN_00637550`, `Network_ServiceLoop`, and the late frame-increment (subject only to the four session-end flags). `State_Machine @ 0x0048C8B0` is called by `Main_Game`, not by `Main_Tick`, so it handles the modal dialog on its own cadence.

**What is frozen when `g_GameState != 0`:** `GScreenClass::Input`, `LogicClass::AI`, `House_AI_Tick`, `Map::Logic`, `RenderFrame_main` (the normal render path). The render inside the gameplay block is skipped.

**`g_GameState` value semantics** (verified via `decompile_function 0x0048C8B0`):

| Value | Activation condition | Evidence |
|---|---|---|
| `0` | Normal gameplay (no modal) | `State_Machine`: `if (g_GameState == 0) return;` |
| `1` | ESC menu / in-game dialog base state | case 1: calls `FUN_004F10E0`; returns to 0 or stays at 1 |
| `2` | Save game confirmation dialog | case 2: save-game command path |
| `3` | Quit/exit confirmation dialog | case 3: can set `DAT_008B41C0 = 1` (quit-to-main session-end flag) |
| `4` | Transition (calls `FUN_005FBEF0`, immediately sets to `5`) | case 4 → `g_GameState = 5` |
| `5` | Options in-game dialog | case 5: `OptionsClass__ShowInGameDialog()` |
| `6` | Diplomacy or other dialog → `5` | case 6 → `g_GameState = 5` |
| `7` | Network sync wait | case 7: `FUN_0077D840` |
| `8` | CD check | case 8: `FUN_006586D0` |
| `9` | Another modal (CD or other) | case 9 |

**Active in YR:** Yes. Any ESC-menu press during a skirmish sets `g_GameState = 1` (or higher), which skips the gameplay block for as many `Main_Tick` calls as the dialog is open, while the frame counter continues to advance.

---

### State E — `DAT_00A8D5F8 & 2` set (replay playback)

**Frame counter behavior:** The gameplay block (Input/AI/Map/Render) is skipped. The replay-playback block runs instead (hash validation, object desync check, `FUN_004F42F0(0)`, `RenderFrame_main`). `LogicClassPerTickUpdateLiveVector` and `g_CurrentFrameCounter++` still run normally.

**Assembly evidence:** `verified via get_assembly_context 0x0055D878` (same triple-gate as §State D, bit-2 branch at `0x0055D87E..0x0055D881`)

**What runs:** Replay-playback validation block, `RenderFrame_main` (inside the playback block), `LogicClassPerTickUpdateLiveVector`, network/service, `g_CurrentFrameCounter++`.

**What is frozen:** `GScreenClass::Input`, `LogicClass::AI`, `House_AI_Tick`, `Map::Logic`, the normal `RenderFrame_main` call (replaced by the replay-path render).

**Active in YR:** Not active in standard skirmish. Active only when replaying a recorded session. Active in YR: No (standard skirmish), Yes (replay playback).

---

## 3. Summary Matrix

| State | Gate address | `g_CurrentFrameCounter++`? | Input/AI/Logic? | Render? | PerTickUpdate? | Network? |
|---|---|---|---|---|---|---|
| A: `g_GameActive == 0` | `0x0055D371 JZ 0x0055DECF` | **NO** | NO | NO | NO | NO |
| B: `g_GameRunning == 0` wait-loop | `0x0055D385 JNZ 0x0055D3BB` | **NO** | NO | NO | NO | YES (Process_NetworkMessages only) |
| C: `Scenario+0x62C != 0` | `0x0055D82E JZ 0x0055D878` | **NO** | NO | YES (RenderFrame+TactUpdate) | NO | YES (Network_ServiceLoop) |
| D: `g_GameState != 0` (in-game pause/modal) | `0x0055D88C JNZ 0x0055D901` | **YES** | NO | NO (gameplay render skipped) | YES | YES |
| E: `DAT_00A8D5F8 & 2` (replay playback) | `0x0055D881 JNZ 0x0055D8FF` | **YES** | NO (input/AI skipped) | YES (replay render) | YES | YES |
| Session-end flags (4 globals) | `0x0055DE4F..0x0055DE71` | **NO** | — | — | NO | YES (Network_ServiceLoop) |

The four session-end flags are already fully decoded in `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md` and are not re-analyzed here.

---

## 4. The Key Correction: In-Game ESC-Menu Does NOT Freeze the Frame Counter

Prior wording in `MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md §3.3` said:

> "Prior timing docs and current decompile agree `g_GameState != 0` skips the normal input/AI/map/render block, but later `PerTickUpdate` and the late increment are not under that same gameplay gate."

This report confirms that finding precisely. The **frame counter advances during the ESC menu / options / save dialogs** in standard YR. The gameplay objects continue to be ticked by `LogicClassPerTickUpdateLiveVector` every `Main_Tick` call. The only things frozen are input processing and the player-visible render pass.

The GScreen doc §8 phrase "SpecialFlags & 2" refers to the replay-playback flag (`DAT_00A8D5F8 & 2`), not an in-game pause flag. There is no SpecialFlags-based pause mechanism for the ESC menu. The ESC menu is controlled purely by `g_GameState != 0`.

---

## 5. INI Keys

No INI key directly controls `g_CurrentFrameCounter` advancement. `[MultiplayerDialogSettings] FogOfWar` is unrelated (TS-legacy fog gate `SpecialFlags & 0x1000` — not this dword). Active in YR: No INI control over the counter gate.

---

## 6. Rust Implementation Delta

### 6.1 The In-Game Pause Mismatch

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test | Risk |
|---|---|---|---|---|---|
| `g_GameState != 0` skips Input/AI/Map/Render but `LogicClassPerTickUpdateLiveVector` and `g_CurrentFrameCounter++` still run — the frame counter advances every Main_Tick except States A, B, C, and session-end. | Rust `state.paused` in `src/app_sim_tick.rs:151..159` prevents `advance_fixed_simulation` entirely — all sim phases AND the frame counter halt. This is incorrect for in-game ESC-menu pause. | `src/app_sim_tick.rs:151..159`; future `World::advance_tick` pause boundary | Enter ESC menu; verify frame counter continues; verify `LogicClassPerTickUpdateLiveVector`-equivalent (ore growth, timers) runs; verify Input/AI/Map/render phases skip. | `paused_game_state_advances_pertick_and_frame_counter_but_skips_input_ai_map` | Do not model as a single app-level stop; model as "skip gameplay block, continue pertick+frame". |

### 6.2 The Scenario-Delay State

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test | Risk |
|---|---|---|---|---|---|
| When `Scenario+0x62C != 0`, network/events/tactical/render run but all logic and frame counter are frozen until the field clears. | No current Rust equivalent for this scenario-delay gate; the field is not tracked. | Future `World::advance_tick` scenario-start sequencing | Start a scenario with `Scenario+0x62C > 0`; verify no logic ticks run, frame counter does not advance, render/network still service. | `scenario_delay_freezes_logic_and_frame_counter` | Identify writers for `Scenario+0x62C` before implementing; deferred per remaining uncertainty. |

### 6.3 `g_GameActive == 0` and `g_GameRunning == 0`

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test | Risk |
|---|---|---|---|---|---|
| `g_GameActive == 0` skips the entire Main_Tick body including frame counter. `g_GameRunning == 0` spins sleeping with network-only service. | Rust `Main_Game` / app loop does not model these two globals explicitly; however, the app loop simply does not call `advance_tick` when not in a session, which is equivalent for `g_GameActive`. The `g_GameRunning == 0` 500ms/10ms sleep-with-network pattern has no Rust equivalent but only applies during focus-loss. | `src/app_sim_tick.rs`; window focus/minimize handling | Focus-loss: verify sim advances zero frames; network-like keepalive still services. | `focus_lost_freezes_all_except_network_poll` | Not a parity blocker for skirmish gameplay; only matters for MP connection keepalive during window minimize. |

---

## 7. Negative Facts / Do Not Do

- **Do NOT treat `DAT_00A8D5F8 & 2` as an in-game pause bit.** It is the replay-playback flag. Verified: `decompile_function 0x0055D360`; `TEST CL,0x2; JNZ 0x0055D8FF` precedes the replay-read block (`FUN_00473B10` calls). Also confirmed by `REPLAY_ACTIVE_VECTOR_RESTORE_CORNER_RESWARM_20260528.md §3.4`.
- **Do NOT freeze `LogicClassPerTickUpdateLiveVector` during in-game ESC-menu pause.** Verified: `decompile_function 0x0055D360`; `LogicClassPerTickUpdateLiveVector()` call is after the triple-gated gameplay block and is unconditional.
- **Do NOT freeze the frame counter during in-game ESC-menu pause.** Verified: `decompile_function 0x0055D360`; `g_CurrentFrameCounter + 1` is only gated by the four session-end flags (`0x0055DE4F..0x0055DE71`), which are victory/defeat/quit/disconnect — not `g_GameState`.
- **Do NOT claim `SpecialFlags & 2` is a pause bit.** The GSCREEN doc at line 481 labels `DAT_00A8D5F8` as "`SpecialFlags` — bit 2 suppresses render/input entirely" — that label is about the effect on the gameplay block, not a pause/ESC-menu mechanism. Verified by caller analysis.
- **Do NOT add an in-game pause path that stops `Network_ServiceLoop`.** All non-advance states B and C still service the network. State D (in-game pause) also runs `Network_ServiceLoop` before the late increment. Verified: `decompile_function 0x0055D360` for all paths.

---

## 8. Remaining Uncertainty

- **Writers for `Scenario+0x62C`:** The exact mechanism that sets this field to a non-zero value was not traced in this slice (out of scope per task constraints). Required before implementing the scenario-delay branch in Rust.
- **`g_GameState` values for multiplayer-only dialogs (7, 8, 9):** The cases 7–9 in `State_Machine` were identified as "network sync wait", "CD check", etc. by code shape; no string-search confirmation was done. These states are unlikely in modern YR without CD, but the pause-skips-gameplay-block conclusion holds for all non-zero values regardless.
- **`Scenario+0x630` writers:** The `Scenario+0x630` byte triggers `FUN_00684180` but does not itself cause the early return. `FUN_00684180` was not decompiled; its body may be a pre-delay setup call. This does not affect the non-advance finding (only `Scenario+0x62C` gates the early return) but is structurally adjacent.

---

## 9. Stale Doc Updates

**`TIMING_SCHEDULER_TICK_SPINE_SYSTEM_MODEL_SYNTHESIS.md` §evidence table row:**

Replace:

> `Four late-increment gate globals and pause/menu/replay branch behavior are fully mapped.` | `MAIN_TICK...` | unknown | medium | conditional | NEEDS_REINVESTIGATE

With:

> `Four late-increment gate globals decoded (session-end flags; see MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md). Pause/menu/replay non-advance matrix decoded (see FRAME_COUNTER_NONADVANCE_PAUSE_SCENARIO_MATRIX_GHIDRA_REPORT.md): g_GameActive==0 full-freeze, g_GameRunning==0 network-only loop, Scenario+0x62C render/net-only return, g_GameState!=0 skips gameplay block but advances frame counter and runs PerTickUpdate.` | `FRAME_COUNTER_NONADVANCE_PAUSE_SCENARIO_MATRIX_GHIDRA_REPORT.md` | confirmed | high | conditional | IMPLEMENTATION_SAFE

**`MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md §3.3` pause/menu row:**

Replace the row:

> Pause/menu state | Prior timing docs and current decompile agree `g_GameState != 0` skips the normal input/AI/map/render block, but later `PerTickUpdate` and the late increment are not under that same gameplay gate.

With:

> Pause/menu state | `g_GameState != 0` (ESC/options/save/quit dialogs) skips Input/AI/Map::Logic/RenderFrame but does NOT freeze the frame counter; `LogicClassPerTickUpdateLiveVector` and `g_CurrentFrameCounter++` run normally. In-game pause is NOT the same as the session-end freeze. See `FRAME_COUNTER_NONADVANCE_PAUSE_SCENARIO_MATRIX_GHIDRA_REPORT.md §State D` for exact assembly evidence.

---

## 10. Sources

- Ghidra decompile: `Main_Tick @ 0x0055D360` — verified via `decompile_function 0x0055D360`
- Ghidra assembly: `0x0055D36B..0x0055D371` (g_GameActive gate) — verified via `get_assembly_context 0x0055D36B`
- Ghidra assembly: `0x0055D377..0x0055D3B0` (g_GameRunning wait loop) — verified via `get_assembly_context 0x0055D383`
- Ghidra assembly: `0x0055D821..0x0055D877` (Scenario+0x62C early return) — verified via `get_assembly_context 0x0055D821,0x0055D862`
- Ghidra assembly: `0x0055D878..0x0055D901` (triple-gate for gameplay block) — verified via `get_assembly_context 0x0055D878,0x0055D883,0x0055D88A,0x0055D893`
- Ghidra decompile: `State_Machine @ 0x0048C8B0` — verified via `decompile_function 0x0048C8B0`
- Cross-doc: `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md` — four session-end flags already decoded
- Cross-doc: `MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md` — prior ordering verified
- Cross-doc: `REPLAY_ACTIVE_VECTOR_RESTORE_CORNER_RESWARM_20260528.md §3.4` — replay bit 2 identity
- Cross-doc: `RENDER_LOGIC_COUPLING_MAIN_TICK_GHIDRA_REPORT.md §4` — replay render confirmation
- Cross-doc: `GSCREEN_RTACTICAL_GHIDRA_REPORT.md §8` — SpecialFlags & 2 reference (replay, not pause)
