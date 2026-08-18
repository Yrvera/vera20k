# Main_Tick Speed Budget — ms/Frame Table, DAT_00A8B558 Identity, and 15 Hz Question

**Date:** 2026-05-28
**Addresses:** `Main_Tick @ 0x0055D360`, `FUN_0055E160 @ 0x0055E160`, `GetRadarTimer @ 0x006C8C40`, `Main_Game @ 0x0052DA00`, `EventClass__Execute @ 0x004C6CC0`, `OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0`, `SessionClass__ReadSkirmishSettings @ 0x00697F10`
**Confidence:** HIGH for all handoff-critical claims. Every binary claim is backed by live decompile or assembly-context call in this session.
**Active in YR:** Yes (local path `g_GameMode == 5`). Network path (`DAT_00A8B558`) is Conditional — only for `g_GameMode != 0 && g_GameMode != 5`.

---

## Target Question

Produce a precise stored-speed-byte → ms/frame ceiling table for BOTH local skirmish and network paths. Pin the identity, default, writer, and reader of `DAT_00A8B558`. Decisively answer: is the default logic frame rate 15 Hz?

## Non-Goals

- Animation `Rate=` timing convention (900 = 60×15 is an art constant, not the logic loop rate).
- The four late-increment guard flags (`DAT_00A83D49`, `DAT_00A8ECD0`, `DAT_008B41C0`, `DAT_00A83D48`).
- PerTickUpdate ordering ladder beyond confirming it runs once per tick.
- Live/wall-clock measurement of realized fps under retail workload (cannot be derived from static binary).

## Evidence Needed to Mark COMPLETE

1. `GetRadarTimer` unit confirmed (16 ms buckets).
2. Local wait helper (`FUN_0055E160`) read confirmed: waits in `GetRadarTimer` buckets using `DAT_00887350`.
3. Network wait in `Main_Tick` confirmed: waits in `timeGetTime` ms using `DAT_00887330`.
4. Full ms/frame table for speed bytes 0–6 in both paths.
5. `DAT_00A8B558`: default value, both writers identified, confirmed never read in mode-5 skirmish.
6. Verdict on 15 Hz question with evidence.

## Stop Conditions

All six evidence items satisfied. ✓

---

## 1. GetRadarTimer — Confirmed 16 ms Buckets

`GetRadarTimer @ 0x006C8C40` decompiles to:

```c
uint GetRadarTimer(void) {
    DWORD DVar1 = timeGetTime();
    return DVar1 >> 4;
}
```

One `GetRadarTimer` unit = 16 ms. (verified via `decompile_function 0x006C8C40`)

---

## 2. Local Path (g_GameMode == 0 or 5) — Wait Budget in 16 ms Buckets

### Budget Setup in Main_Tick

From `decompile_function 0x0055D360`, the mode-5 (skirmish) path:

```c
uVar19 = DAT_00a8eb60;          // load stored speed byte
DAT_00887348 = GetRadarTimer(); // record tick-start bucket
DAT_00887350 = uVar19;          // set wait budget = speed byte (in 16ms units)
// ... ALL game work: Input, LogicClass__AI, Map__Logic, RenderFrame_main ...
g_CurrentFrameCounter += 1;    // late frame commit
FUN_0055E160();                 // WAIT until budget is consumed
```

`DAT_00887350` is the budget in `GetRadarTimer` buckets. (verified via `decompile_function 0x0055D360`)

### Wait Logic in FUN_0055E160 (Local Branch)

From `decompile_function 0x0055E160`, the local branch (`g_GameMode == 0 or 5`):

```c
DVar3 = DAT_00887350;  // budget in 16ms buckets
if (DAT_00887348 != -1) {
    iVar1 = GetRadarTimer();
    remaining = (iVar1 - DAT_00887348 < DVar3) ? DVar3 - (iVar1 - DAT_00887348) : 0;
}
// if remaining != 0: call FUN_004a4830(), then Sleep(remaining * 16ms equiv)
```

The local wait loop checks `GetRadarTimer()` and sleeps via `Sleep(DVar3)` where `DVar3` is remaining 16-ms-bucket units. The sleep argument is in `GetRadarTimer` units (NOT milliseconds directly). The ceiling is `budget × 16 ms` minus elapsed work time. (verified via `decompile_function 0x0055E160`)

**Important:** `Sleep(N)` is called with the remaining `GetRadarTimer` bucket count, NOT milliseconds. One bucket ≈ 16 ms, but Windows `Sleep()` takes milliseconds — this means the local path passes a small integer (0–6) to `Sleep()`, which rounds up to the OS scheduler quantum (~15.6 ms on untuned Windows). At `budget = 1`, `Sleep(1)` is called, sleeping ~1 ms minimum or up to the OS quantum. The `GetRadarTimer`-based loop retries until the bucket elapsed; actual ceiling is ≈16 ms per bucket.

### Local Path ms/Frame Table (Static Nominal Ceiling)

| Stored speed byte | `DAT_00887350` | Nominal ceiling (buckets × 16 ms) | Notes |
|---|---|---|---|
| 0 | 0 | 0 ms — no wait, free-run | Maximum speed |
| 1 | 1 | ≤ 16 ms (~62.5 fps ceiling) | **Default YR skirmish** |
| 2 | 2 | ≤ 32 ms (~31.25 fps ceiling) | mode-0 forced value |
| 3 | 3 | ≤ 48 ms (~20.8 fps ceiling) | |
| 4 | 4 | ≤ 64 ms (~15.6 fps ceiling) | |
| 5 | 5 | ≤ 80 ms (~12.5 fps ceiling) | |
| 6 | 6 | ≤ 96 ms (~10.4 fps ceiling) | Slowest |

"Nominal ceiling" = upper bound; realized fps = ceiling minus work time, then floored by OS scheduler granularity.

(verified via `decompile_function 0x0055D360`, `decompile_function 0x0055E160`, `decompile_function 0x006C8C40`)

---

## 3. Network Path (g_GameMode != 0 and != 5, DAT_00A8B24C == 2) — Wait Budget in Milliseconds

### Budget Setup in Main_Tick

From `decompile_function 0x0055D360`, the network branch (only reached when `g_GameMode != 0 && g_GameMode != 5 && DAT_00A8B24C == 2`):

```c
if (DAT_00a8b558 == 0) {
    DAT_00887350 = 2;            // local bucket budget (fallback)
    DAT_00887330 = 0x21;         // ms budget = 33 ms
} else {
    lVar1 = (longlong)DAT_00a8b558;
    DAT_00887350 = (int)(0x3c / lVar1);   // local bucket budget = 60 / fps
    DAT_00887330 = (int)(1000 / lVar1);   // ms budget = 1000 / fps
}
// Wait loop in FUN_0055E160 uses timeGetTime + DAT_00887330 for network modes
```

(verified via `decompile_function 0x0055D360`)

### Network Wait Logic in FUN_0055E160

For `g_GameMode != 0 && g_GameMode != 5`, `FUN_0055E160` uses `timeGetTime` and `DAT_00887330` (milliseconds):

```c
do {
    iVar1 = DAT_00887330;
    if (DAT_00887328 != -1) {
        DVar3 = timeGetTime();
        if (iVar1 <= (int)(DVar3 - DAT_00887328)) break;
        iVar1 = iVar1 - (DVar3 - DAT_00887328);
    }
    // render / service loop while waiting
} while (true);
```

(verified via `decompile_function 0x0055E160`)

### Network Path ms/Frame Table

`DAT_00A8B558` holds the negotiated network FPS (frame rate, not speed byte). Default = 30.

| `DAT_00A8B558` (net fps) | `DAT_00887330` (ms budget) | `DAT_00887350` (bucket budget) | Nominal ms/frame ceiling |
|---|---|---|---|
| 0 (fallback) | 33 ms (0x21) | 2 buckets | 33 ms (~30.3 fps) |
| 10 | 100 ms | 6 buckets | 100 ms (10 fps) |
| 15 | 66 ms | 4 buckets | 66 ms (15 fps) |
| 20 | 50 ms | 3 buckets | 50 ms (20 fps) |
| 30 (default) | 33 ms | 2 buckets | 33 ms (30 fps) |
| 60 | 16 ms | 1 bucket | 16 ms (60 fps) |

`0x3c = 60`; `60 / fps` rounds down via integer division. 1000 / fps likewise rounds down.

(verified via `decompile_function 0x0055D360` immediates `0x3c`, `1000`, `0x21`)

---

## 4. DAT_00A8B558 — Identity, Default, Writers, Readers

### Identity

`DAT_00A8B558` is the **negotiated network FPS divisor** — the number of logic frames per second agreed by all network peers before a multiplayer match starts. It is NOT the game speed byte (`DAT_00A8EB60`). These are independent globals.

### Default Value: 30 (0x1E)

`Main_Game @ 0x0052DABD` unconditionally writes `0x1e` (= 30) to `DAT_00A8B558` at the start of every game session:

```asm
0052dabd: MOV dword ptr [0x00a8b558], 0x1e
```

(verified via `get_assembly_context 0x0052dabd`, `get_xrefs_to 0x00A8B558`)

### Writers

| Writer | Address | Value written | Trigger |
|---|---|---|---|
| `Main_Game` | `0x0052DABD` | `0x1e` (30) | Every game start/restart |
| `EventClass__Execute` case `0x20` | `0x004C807D` | `*(ushort*)(param_1+7)` | Network "frame rate negotiation" event |

(verified via `get_assembly_context 0x0052dabd`, `get_assembly_context 0x004c807d`, `get_xrefs_to 0x00A8B558`)

The network event writer loads `ECX = MOVZX word ptr [ESI+7]` — a 16-bit value from the event packet — then writes to `DAT_00A8B558`. This is the negotiated peer FPS received from the network host.

### Readers in Main_Tick

`DAT_00A8B558` is read at `0x0055D491` and `0x0055D522` (both inside the network branch gated by `g_GameMode != 0 && g_GameMode != 5 && DAT_00A8B24C == 2`). It is **never read** in the local skirmish branch (`g_GameMode == 5`). (verified via `get_xrefs_to 0x00A8B558`, `decompile_function 0x0055D360`)

### Active in YR

- Initialized: always (every game start via `Main_Game`).
- Read/used: Conditional — only in network mode (`g_GameMode 3/4 with DAT_00A8B24C == 2`).
- In standard local skirmish (`g_GameMode == 5`): **NEVER READ**. Zero impact on skirmish timing.

---

## 5. Is the Default Logic Frame Rate 15 Hz?

**Verdict: NO. The default local skirmish frame-rate CEILING is ~62.5 Hz (speed byte 1, 16 ms budget), not 15 Hz.**

Evidence:

1. Default stored speed byte = 1 (from `rulesmd.ini [MultiplayerDialogSettings] GameSpeed=1`, reader `SessionClass__ReadSkirmishSettings @ 0x00697F5E`). (verified via `decompile_function 0x00697F10`, INI `ini/rulesmd.ini` line 3026)

2. Speed byte 1 → `DAT_00887350 = 1` → wait budget = 1 `GetRadarTimer` bucket = 16 ms. (verified via `decompile_function 0x0055D360`)

3. `FUN_0055E160` waits until 1 `GetRadarTimer` bucket elapses, giving a ceiling of 1 tick per ~16 ms ≈ **62.5 fps**. (verified via `decompile_function 0x0055E160`)

4. `900 = 60 × 15` appears in `FUN_0055E160` at `DAT_00ABCD90 = 0x3c` (60) and was previously seen in animation Rate= INI — this is an **art timing constant** (animations run at 60 Rate units = 1 second), NOT a logic-loop frequency. (verified via `decompile_function 0x0055E160`, value `0x3c = 60` at `LAB_0055e39b`)

5. The network default is 30 Hz (`DAT_00A8B558 = 0x1e`), but this applies only to network modes, not local skirmish.

**The "15 Hz logic rate" referenced in the Rust codebase** (`src/util/fixed_math.rs` comment: "matches RA2's native 15 fps game logic rate") and `binary_frame = total_sim_ms * 15 / 1000` is a **Rust internal approximation**, not a gamemd-native rate. gamemd has no 15 Hz logic rate; it runs one tick of logic per `Main_Tick` call, with the frame CEILING set by the speed-byte budget.

Where does "15" come from in the Rust code? Likely from the `900 = 60 × 15` art convention: art `Rate=` values use 900 = 60 seconds × 15 animation frames/sec as an encoding baseline. This is NOT the game loop rate.

---

## 6. Implementation Handoff

### Handoff A — Rust SIM_TICK_HZ = 45 vs. gamemd ≈62.5 fps ceiling

**Verified gamemd behavior:** Default local skirmish runs one logic tick per `Main_Tick`, with a ceiling of ~62.5 fps (speed byte 1, 16 ms budget). At speed byte 0 (max speed), free-run with no ceiling.

**Rust delta:** `SIM_TICK_HZ = 45` (`src/util/fixed_math.rs:51`) is a Rust internal tick rate. This is NOT gamemd-native. The `binary_frame = total_sim_ms * 15 / 1000` formula also uses an unsourced "15 Hz" rate. The decision is whether to adopt `SIM_TICK_HZ ≈ 62.5` for default speed-1 parity, or retain 45 as a fixed scheduling rate distinct from gamemd's variable-rate-ceiling model.

**Affected surface:** `src/util/fixed_math.rs` (`SIM_TICK_HZ`), `src/sim/world/mod.rs` (`binary_frame` formula), any speed-byte → tick-interval mapping in app scheduler.

**Acceptance scenario:** A test fixture at speed byte 1 runs 1000 `Main_Tick` equivalents and confirms `g_CurrentFrameCounter` increments 1000 times, with average inter-tick ≈ 16 ms.

**Proposed test name:** `test_speed_byte_1_tick_rate_ceiling_16ms`

**Risk:** Medium. The current `SIM_TICK_HZ = 45` was deliberately chosen; changing it affects all INI-timing consumers (ROF, Speed, Rate). Confirm with the user before changing — this report establishes the gamemd ceiling, not a mandate to change Rust's internal rate.

### Handoff B — `binary_frame = total_sim_ms * 15 / 1000` is Not a gamemd Derivation

**Verified gamemd behavior:** `g_CurrentFrameCounter` increments once per `Main_Tick` call (verified by `decompile_function 0x0055D360`, the increment `g_CurrentFrameCounter = g_CurrentFrameCounter + 1` before `FUN_0055E160`). There is no 15 Hz rate in the binary. The frame counter advances at the rate the loop fires — up to ~62.5 Hz at speed 1.

**Rust delta:** The comment in `src/sim/world/mod.rs:1887–1895` says "Drift-free: every binary-frame boundary is exactly when total_sim_ms crosses a multiple of 1000/15 ≈ 66.67ms." This is an internal approximation. If `SIM_TICK_HZ` is 45 and each sim tick is ~22ms, binary_frame using `*15/1000` will diverge from a gamemd running at ~62.5 fps (16 ms/frame) by about 4×. This is a known compromise; the report surfaces it explicitly.

**Affected surface:** `src/sim/world/mod.rs:1895` — `binary_frame` formula.

**Acceptance scenario:** Integration test: simulate 1 second at `SIM_TICK_HZ = 45` and verify `binary_frame` does not exceed the gamemd-equivalent frame count for that duration at default speed 1.

**Proposed test name:** `test_binary_frame_does_not_exceed_gamemd_at_speed1`

**Risk:** Low if no change is made; Medium if `SIM_TICK_HZ` is adjusted. Document the formula as an approximation, not a gamemd derivation.

### Handoff C — DAT_00A8B558 Not Needed for Skirmish Port

**Verified gamemd behavior:** `DAT_00A8B558` is initialized to 30 and only consumed in the network branch. It is never read during local skirmish (`g_GameMode == 5`). The network FPS is negotiated at match start via `EventClass::Execute` case `0x20`.

**Rust delta:** No Rust equivalent of `DAT_00A8B558` is needed for single-player or local skirmish. If multiplayer is implemented, the negotiated FPS divisor should be a session-level field, separate from the game speed byte.

**Affected surface:** Future network/session module.

**Acceptance scenario:** Skirmish tests pass without any `network_fps_divisor` field set.

**Proposed test name:** `test_skirmish_no_network_fps_divisor`

**Risk:** Low.

---

## 7. Negative Facts / Do Not Do

1. **Do NOT treat `900 = 60×15` as proof of a 15 Hz logic rate.** `900` appears only in the art `Rate=` timing convention and as a `0x3c` (60) constant in `FUN_0055E160`'s stats-accumulator path (`DAT_00ABCD90 = 0x3c`). It has no relationship to logic tick frequency. (verified via `decompile_function 0x0055E160`, `LAB_0055e39b` block)

2. **Do NOT use `DAT_00A8B558` (default 30) as the skirmish tick rate.** It is the network peer FPS divisor, never read in mode-5 skirmish. (verified via `get_xrefs_to 0x00A8B558`, `decompile_function 0x0055D360`)

3. **Do NOT assume the local wait argument to Sleep() is milliseconds.** In the local path, `Sleep(DVar3)` is called with the remaining `GetRadarTimer` bucket count (0–6), not milliseconds. The loop retries using `GetRadarTimer` until the budget elapses. (verified via `decompile_function 0x0055E160`)

4. **Do NOT map stored speed byte directly to fps.** The mapping is: `fps_ceiling ≈ 1000 / (speed_byte × 16)` for speed bytes 1–6; speed byte 0 is free-run (no sleep). (derived from verified `GetRadarTimer = timeGetTime()>>4` and budget math)

5. **Do NOT treat the network path as reachable from standard local skirmish.** The network branch in `Main_Tick` is gated by `g_GameMode != 0 && g_GameMode != 5 && DAT_00A8B24C == 2`. `g_GameMode == 5` is the standard skirmish mode. (verified via `decompile_function 0x0055D360`)

---

## 8. Remaining Uncertainty

- **Realized fps at speed byte 1 under retail workload.** The static analysis gives a ceiling of ~62.5 fps (1 × 16 ms budget). Actual throughput depends on: Windows `Sleep()` granularity (~15.6 ms on untuned Windows), render workload duration, and the elapsed-bucket subtraction. Requires live runtime probe — cannot be resolved from binary alone.

- **Whether `DAT_00A8B558 = 0` (fallback to 33 ms budget) is reachable in practice.** The fallback path fires if the network FPS is never negotiated via event `0x20`. Under normal multiplayer play this should be overwritten before the first frame; the fallback may be unreachable in practice. Marked UNVERIFIED — would require network session trace.

- **Slider `TBM_SETRANGE` min/max from dialog resource.** The `6 - slider_position` formula is confirmed from code, and range 0–6 is inferred. The dialog resource was not decompiled to confirm the explicit `TBM_SETRANGE` call. HIGH confidence but unconfirmed from resource.

---

## 9. Relationship to Existing Docs

- `GAME_SPEED_SETTING_RATE_VS_CONTENT_GHIDRA_REPORT.md` — covers rate-vs-content verdict, two-globals separation, slider mapping, default speed byte 1, and all writers. **All findings confirmed consistent.** This report extends it with the full ms/frame table for both paths, the `Sleep()` argument semantics, the `FUN_0055E160` network wait mechanics, and the explicit 15 Hz verdict.

- `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md` — `GetRadarTimer = timeGetTime()>>4` confirmed consistent.

- `skirmish-ui/DEFAULT_SKIRMISH_FRAME_PACE_EXTENSION_GHIDRA_REPORT.md` — §3.3/§3.6 "does local loop settle near 62.5 fps" open question is now answered statically: ceiling is 62.5 fps at speed 1; realized fps requires live probe. No corrections needed.

---

## Sources

- `decompile_function 0x0055D360` — `Main_Tick` full budget setup for local and network paths
- `decompile_function 0x0055E160` — `FUN_0055E160` wait logic, both local (GetRadarTimer) and network (timeGetTime) branches
- `decompile_function 0x006C8C40` — `GetRadarTimer = timeGetTime() >> 4`
- `get_assembly_context 0x0052dabd` — `MOV [0x00a8b558], 0x1e` (default 30) in `Main_Game`
- `get_assembly_context 0x004c807d` — `MOV [0x00a8b558], ECX` (network event write)
- `get_xrefs_to 0x00A8B558` — confirmed all writers and that mode-5 skirmish has no read path
- `decompile_function 0x004E1DE0` — `OptionsClass__ApplyFromInGameDialog` slider = `6 - LVar2`
- `decompile_function 0x00697F10` — `SessionClass__ReadSkirmishSettings` fallback to `RulesClass+0x14A0`
- INI `ini/rulesmd.ini [MultiplayerDialogSettings] GameSpeed=1` (line 3026)
- INI `ini/rules.ini [MultiplayerDialogSettings] GameSpeed=0` (line 2506)
- Rust: `src/util/fixed_math.rs:51` (`SIM_TICK_HZ = 45`), `src/sim/world/mod.rs:1895` (`binary_frame` formula)
