# Live Skirmish Pacing Path — Ghidra Report

**Date:** 2026-05-28
**Addresses:** `Main_Tick @ 0x0055D360`, `FUN_0055E160 @ 0x0055E160`, `GetRadarTimer @ 0x006C8C40`
**Confidence:** High (all key claims verified via live Ghidra decompile this session)
**Active in YR:** Yes — standard local skirmish path (`g_GameMode == 5`)

---

## Investigation Scope

**Target question:** Which `Main_Tick` branch executes for local skirmish (`g_GameMode == 5`), what is the throttle budget, and what is the resulting ms per `g_CurrentFrameCounter` increment at the default skirmish speed?

**Non-goals:** The four increment-guard flags in detail; the game-speed setting source and range; animation/movement frame-basis; particle normalization.

**Evidence needed to mark COMPLETE:** Live Ghidra decompile of `Main_Tick @ 0x0055D360` and `FUN_0055E160 @ 0x0055E160` confirming branch structure, budget value, and sleep arithmetic. INI confirmation of default speed byte.

**Stop conditions:** Branch structure confirmed, budget extracted, sleep formula verified, live-vs-network contrast explicit.

---

## 1. Branch Identification — g_GameMode == 5

`Main_Tick` first saves `uVar19 = DAT_00a8eb60` (the live speed byte), then branches:

```text
if (g_GameMode == 0) {
    if (DAT_00a8eddc == '\0') {
        DAT_00a8eb60 = 2;          // mode-0 speed-2 override (NOT skirmish)
        DAT_00887348 = GetRadarTimer();
        DAT_00887350 = 2;
        goto LAB_0055d7c2;
    }
    goto LAB_0055d79e;             // mode-0 without override
} else {
    if ((g_GameMode == 5) || (DAT_00a8b24c != 2)) goto LAB_0055d79e;
    // ... NETWORK/REPLAY BLOCK (see §2) ...
}

LAB_0055d79e:
    DAT_00887348 = GetRadarTimer();
    DAT_00887350 = uVar19;         // = live speed byte = 1 for default skirmish
    goto LAB_0055d7c2;
```

**Finding:** For `g_GameMode == 5`, the condition `(g_GameMode == 5) || (DAT_00a8b24c != 2)` is true immediately, so `goto LAB_0055d79e` fires. The network/replay block is **never entered**. `LAB_0055d79e` sets `DAT_00887348 = GetRadarTimer()` and `DAT_00887350 = DAT_00a8eb60` (the live speed byte, `1` at default).

Active in YR: **Yes**. Evidence: verified via `decompile_function 0x0055D360`.

---

## 2. Network/Replay Block — What It Is and Why It Is NOT Skirmish

The block between `else { ... }` (reached only when `g_GameMode != 0 AND g_GameMode != 5 AND DAT_00a8b24c == 2`) is the network/replay pacing path. It contains:

```text
if (DAT_00a8b558 == 0) {
    DAT_00887350 = 2;
    local_1ac = 0x21;              // 33 ms network budget
} else {
    DAT_00887350 = (int)(0x3c / DAT_00a8b558);   // radar-bucket budget
    local_1ac = (int)(1000 / DAT_00a8b558);      // ms budget
}
DAT_00887328 = timeGetTime();      // ms-based timer start (NOT GetRadarTimer)
DAT_00887330 = local_1ac;          // ms budget for FUN_0055E160 network path
FUN_005d5870(); FUN_005d5880();

if (g_GameMode != 4) goto LAB_0055d7c2;
// mode-4 only: +10ms bumps based on g_NetworkFrameBudget thresholds (1/4, 1/2, 3/4)
```

The `1000 / DAT_00a8b558` and `0x3c / DAT_00a8b558` formulas, `FUN_005d5870()`, `FUN_005d5880()`, and the mode-4 budget bumps all live here. **This block has zero reachability for `g_GameMode == 5`.**

Active in YR: **Conditional** (network/multiplayer/replay modes only). Evidence: verified via `decompile_function 0x0055D360`.

---

## 3. Throttle Budget for Default Skirmish

`GetRadarTimer @ 0x006C8C40` returns `timeGetTime() >> 4`. One tick of `GetRadarTimer()` represents a 16 ms wall-clock bucket. Evidence: verified via `decompile_function 0x006C8C40` (prior session, noted in GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md §3.2 and confirmed unchanged).

For default YR skirmish:
- `DAT_00a8eb60` = `1` (sourced from `rulesmd.ini [MultiplayerDialogSettings] GameSpeed=1` via `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`; verified via INI `ini/rulesmd.ini`)
- `DAT_00887350 = 1` (one `GetRadarTimer()` bucket)
- `DAT_00887348 = GetRadarTimer()` (snapshot before the tick's work runs)

Evidence: verified via `decompile_function 0x0055D360` — `LAB_0055d79e` path assignments.

---

## 4. FUN_0055E160 Sleep Path (Modes 0 and 5)

The throttle helper (`FUN_0055E160 @ 0x0055E160`) structure for modes 0 and 5:

```text
DVar3 = DAT_00887350;          // load budget (= 1 bucket for default skirmish)
if (DAT_00887348 != -1) {
    iVar1 = GetRadarTimer();
    if (iVar1 - DAT_00887348 < (int)DVar3)
        DVar3 = DVar3 - (iVar1 - DAT_00887348);   // subtract elapsed work buckets
    else
        DVar3 = 0;             // work consumed full budget, no sleep needed
}

// Only for g_GameMode != 0 AND != 5: network path (timeGetTime() ms loop)
if ((g_GameMode != 0) && (g_GameMode != 5)) { /* network loop */ }

// Local skirmish / mode-0 path:
if (DAT_00887348 == -1) {
    if (DVar4 != 0) {
        FUN_004a4830();
        // ... retry loop ...
        Sleep(DVar4 - (GetRadarTimer() - DAT_00887348));
    }
} else {
    iVar1 = GetRadarTimer();
    if (iVar1 - DAT_00887348 < (int)DVar4) {
        DVar4 = DVar4 - (iVar1 - DAT_00887348);
        // loops back to Sleep branch:
        Sleep(DVar4 - (GetRadarTimer() - DAT_00887348));
    }
}
```

**Key math:** `Sleep` argument = `budget_buckets - elapsed_buckets`. Since 1 bucket = 16 ms: if work took < 1 bucket (< 16 ms), the helper sleeps the remaining fraction. If work took ≥ 1 bucket, sleep = 0. 

`DAT_00887328` (the `timeGetTime()` ms start) is **never set** for modes 0 or 5. The network path inside `FUN_0055E160` is guarded `if ((g_GameMode != 0) && (g_GameMode != 5))` and does not execute for local skirmish.

Evidence: verified via `decompile_function 0x0055E160`.

---

## 5. Resulting ms per g_CurrentFrameCounter Increment

**Nominal (static):** Budget = 1 `GetRadarTimer()` bucket = 16 ms. The helper sleeps until the budget is consumed. If work takes T ms, the helper sleeps max(16 - T, 0) ms. Total frame wall-clock = max(T, 16) ms.

**Effective frame cap (nominal):** ~16 ms/increment = **~62.5 Hz** at default speed byte 1, when render workload is below 16 ms.

**Important qualification:** This is the static throttle contract. The realized rate under retail load depends on:
1. Windows `Sleep()` granularity (typically 15.6 ms at default timer resolution, sometimes less after `timeBeginPeriod(1)` — `Main_Tick` stores `timeGetTime()` at `_DAT_00a8b55c` but no `timeBeginPeriod` call was found at this address; the effect is unverified without a live attached process).
2. Render workload: if a frame's `LogicClass__AI + RenderFrame_main + etc.` exceeds 16 ms, the sleep is 0 and the realized rate drops below 62.5 Hz.
3. The `FUN_004a4830()` call inside the bucket-wait retry loop (address `0x004a4830`) — its behavior is not verified in this session; it may perform additional work per wait cycle.

**15 Hz is NOT the throttle rate.** The `900 / Rate` art convention uses 15 as a content-calibration constant (`60 sec * 15 frames/min`), but the throttle bucket is 16 ms → ~62.5 Hz. These are separate numbers that happen to be near each other.

**The art convention (15 fps) and the throttle rate (~62.5 fps at speed 1) are deliberately different.** AnimClass frame advances use game frames; at 62.5 Hz with `Rate=1` (internal delay = 0), animations advance every tick. At `Rate=60` (internal delay = 15), an animation advances every ~15 ticks = ~240 ms wall-clock ≈ one second at 62.5 Hz, which matches the art "15 frames/sec" convention for `900/60 = 15`-frame delays.

Active in YR: **Yes**. Evidence: verified via `decompile_function 0x0055D360` + `decompile_function 0x0055E160`.

---

## 6. Explicit Live vs. Network Throttle Contrast

| Property | **Local Skirmish (g_GameMode == 5)** | **Network/Replay (g_GameMode != 0/5 AND DAT_00a8b24c == 2)** |
|---|---|---|
| Budget global | `DAT_00887350` (radar-bucket units) | `DAT_00887330` (millisecond units) |
| Start timer | `DAT_00887348 = GetRadarTimer()` | `DAT_00887328 = timeGetTime()` |
| Budget value (default) | `1` bucket = ~16 ms | `1000 / DAT_00a8b558` ms (e.g. `1000/30 = 33` ms at 30 fps network) |
| Budget divisor | Speed byte from `DAT_00a8eb60` | `DAT_00a8b558` (requested network FPS) |
| `0x3c /` formula | **Not used** | `0x3c / DAT_00a8b558` populates `DAT_00887350` (secondary) |
| +10ms budget bumps | **Not applied** | Applied up to 3× for `g_GameMode == 4` based on `g_NetworkFrameBudget` thresholds |
| `FUN_005d5870/5880` | **Not called** | Called on this path |
| FUN_0055E160 path | Local bucket-Sleep path | Network `timeGetTime()` loop with `Sleep(0)` + possible re-render |

---

## 7. g_CurrentFrameCounter Increment Order

Within `Main_Tick`, for the normal active path, the increment fires after:
1. `GScreenClass__Input`
2. `LogicClass__AI`
3. optional `House_AI_Tick`
4. `Map__Logic`
5. `RenderFrame_main`
6. `FUN_00551A30` (side work)
7. `LogicClassPerTickUpdateLiveVector()`
8. Sound/hash/defeat-detection routines
9. `Network_ServiceLoop()`
10. Four-guard check → `g_CurrentFrameCounter += 1`
11. `FUN_0055E160` (sleep/throttle)

Systems that read `g_CurrentFrameCounter` during steps 1–9 see the **old** value (frame N). The increment and sleep complete at the very end of the tick.

Evidence: verified via `decompile_function 0x0055D360` — increment at end of function after `Network_ServiceLoop()`.

---

## 8. Remaining Uncertainty

- **Realized hardware rate:** The static throttle contract says ~16 ms/frame = ~62.5 Hz. The realized rate under actual Windows `Sleep()` granularity (without `timeBeginPeriod(1)`) may be closer to ~15.6 ms (64 Hz) or lower. No live attached process confirmed the actual sampled rate; this is a YELLOW unverified claim.
- **`FUN_004a4830 @ 0x004a4830`:** Called inside the local-mode wait retry loop. Its behavior was not decompiled this session. It may process messages, yield, or perform additional wait work. Does not change the nominal throttle math but could affect the realized rate.
- **Mode-0 without `DAT_00a8eddc`:** When `g_GameMode == 0` and `DAT_00a8eddc == '\0'`, the speed byte is forced to `2` and `DAT_00887350 = 2` (two buckets = ~32 ms). This path is **not** standard local skirmish (`g_GameMode == 5`); its exact trigger condition is unverified beyond the decompile branch.
- **`DAT_00abcd90 = 0x3c` at end of `FUN_0055E160`:** After the sleep, the helper writes `0x3c` to `DAT_00abcd90` and resets `DAT_00abcd88 = GetRadarTimer()`. This appears to be a 60-bucket (~960 ms) secondary timer used by the perf-stats block at `LAB_0055e39b`. Its effect on pacing is not material to the per-frame throttle, but it is noted as unverified.

---

## 9. Implementation Handoff

### Handoff 1 — Frame rate is ~62.5 Hz, not 15 Hz or 45 Hz (skirmish speed 1)
- **Verified behavior:** `DAT_00887350 = 1` (one 16-ms bucket) for default skirmish; `FUN_0055E160` sleeps to fill that budget. Nominal ~16 ms/frame = ~62.5 Hz.
- **Rust delta:** `src/sim/world/mod.rs` line ~1014-1015 derives `binary_frame = total_sim_ms * 15 / 1000`. This formula gives one `binary_frame` advance per 66.7 ms — roughly 4× slower than the gamemd throttle. Any system wired to `binary_frame` increments at ~15 Hz, not ~62.5 Hz. The formula should be `total_sim_ms * 62_5 / 1000` (integer approximation: `total_sim_ms * 1000 / 16000` or `total_sim_ms / 16`). Using `/ 16` loses fractional precision; exact match requires `total_sim_ms * 1000 / 16_000` with care for overflow.
- **Affected surface:** Every consumer of `binary_frame` in `src/sim/world/mod.rs` — CDTimer emulation, AnimClass emulation, WalkRate/IdleRate gates.
- **Acceptance scenario:** In a default skirmish, `binary_frame` should advance by ~62 units per wall-clock second (not ~15). A 1-second probe with no pauses must produce `binary_frame_delta ∈ [58, 68]`.
- **Proposed test:** `test_binary_frame_advances_at_62hz_per_wall_second`
- **Risk:** All existing tests and frame-based constants calibrated against 15 Hz will produce wrong timing until updated.

### Handoff 2 — Skirmish uses radar-bucket throttle, not ms throttle; budgets are NOT additive across ticks
- **Verified behavior:** `DAT_00887348` is reset to `GetRadarTimer()` at the START of every `Main_Tick`, before work begins. `FUN_0055E160` subtracts elapsed work time before sleeping. Each tick is a self-contained 1-bucket window.
- **Rust delta:** `src/app_sim_tick.rs` uses an elapsed-wall-time scheduler (`sim_speed_tps / SIM_TICK_HZ` scaling). This matches the bucket semantics if `sim_speed_tps` is calibrated to ~62.5, but the bucket mechanism is work-subtracted, not fixed-step. If the app's fixed-step scheduler runs 45 steps/sec and each step takes < 22 ms, the pacing differs from gamemd's per-tick budget.
- **Affected surface:** `src/app_sim_tick.rs` scheduler, `src/app_types.rs` TPS computation.
- **Acceptance scenario:** Scheduling 1 advance_tick per 16 ms wall-clock window (work-subtracted) matches gamemd. Test: place a 10-ms artificial work delay inside advance_tick; the resulting frame rate should still be ~62.5 Hz (budget fills the remaining 6 ms), not a visible slowdown.
- **Proposed test:** `test_work_subtracted_throttle_maintains_62hz`
- **Risk:** Low — this is a scheduler policy, not a sim invariant. Safe to fix independently of game logic.

### Handoff 3 — Network/replay throttle block is unreachable for g_GameMode == 5; do not mix the two paths
- **Verified behavior:** The `0x3c / DAT_00a8b558` and `1000 / DAT_00a8b558` formulas, `DAT_00887328/00887330`, and `FUN_005d5870/5880` are all inside a block gated by `g_GameMode != 0 AND g_GameMode != 5 AND DAT_00a8b24c == 2`. They cannot fire for standard local skirmish.
- **Rust delta:** If any Rust code uses a `1000 / fps`-style budget for the local skirmish path, it is implementing the network path, not the skirmish path.
- **Affected surface:** Any Rust pacing/throttle in `src/app_sim_tick.rs` that uses an ms-based budget derived from a target FPS.
- **Acceptance scenario:** Local skirmish pacing must use a bucket-based approach where 1 bucket = 16 ms and budget = `game_speed_byte` buckets. Network pacing uses a separate ms budget derived from the negotiated frame rate.
- **Proposed test:** `test_skirmish_default_speed_frame_period_ms` (unit: assert nominal period = 16 ms for speed byte 1)
- **Risk:** Medium — conflating the two paths causes the local skirmish to use wrong budget units under load.

---

## 10. Negative Facts / Do Not Do

1. **Do not use `1000 / DAT_00a8b558` as the skirmish frame budget.** That formula lives in the network/replay block (`g_GameMode != 0/5, DAT_00a8b24c == 2`), not the skirmish path. Evidence: verified via `decompile_function 0x0055D360` — the formula is inside the `else` branch unreachable for `g_GameMode == 5`.

2. **Do not treat the `900/Rate` art constant (15 fps convention) as the throttle rate.** `900 = 60 * 15` is a content-calibration constant for art durations in frames-per-minute. The throttle is 16 ms buckets (~62.5 Hz at speed 1). These are separate numbers. Evidence: `AnimTypeClass__ReadINI @ 0x00427D00` (art constant), `GetRadarTimer @ 0x006C8C40` (throttle).

3. **Do not apply the mode-0 speed-2 override to g_GameMode == 5.** The branch `if (g_GameMode == 0) { if (DAT_00a8eddc == '\0') { DAT_00887350 = 2; ... } }` is gated on `g_GameMode == 0`, not `5`. Evidence: verified via `decompile_function 0x0055D360` — the condition is `g_GameMode == 0`.

4. **Do not set `DAT_00887328` (timeGetTime ms start) for the skirmish path.** That global is only set in the network/replay block. `FUN_0055E160` uses `DAT_00887328` only in its network branch (`g_GameMode != 0 AND != 5`). Evidence: verified via `decompile_function 0x0055E160` — the `DAT_00887328` read is inside `if ((g_GameMode != 0) && (g_GameMode != 5))`.

5. **Do not apply mode-4 +10ms budget bumps to skirmish.** The three `DAT_00887330 += 10` increments are inside `if (g_GameMode != 4) goto LAB_0055d7c2;` — they require `g_GameMode == 4`. Evidence: verified via `decompile_function 0x0055D360`.

---

## 11. Stale-Doc Replacement Findings

**GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md §3.1** states:
> "In normal local paths (`g_GameMode == 0` or `5`), it sets: `DAT_00887348 = GetRadarTimer()` / `DAT_00887350 = DAT_00A8EB60`"

This is correct and is confirmed. No replacement needed.

**GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md §8 Open Question 1:**
> "What is the measured retail wall-clock `g_CurrentFrameCounter` delta/sec in a default local YR skirmish..."

The static answer is now: **nominal ~62.5 Hz (1 bucket = 16 ms at speed 1)**. A live measurement is still needed to confirm realized rate under Windows Sleep granularity, but the static throttle contract is now verified.

**DEFAULT_SKIRMISH_FRAME_PACE_EXTENSION_GHIDRA_REPORT.md §3.4** states:
> "With normal local setup, `DAT_00887348` is set from `GetRadarTimer()` before game work, so elapsed work time is subtracted from the speed budget."
> "Static decompilation gives a nominal speed-1 throttle of one 16 ms bucket..."

This is confirmed and now has stronger inline evidence (live decompile this session). The report does not explicitly state the ~62.5 Hz number; this report adds it.

**docs/index.md** states the sim runs at 45 ticks/sec and calls 45 FPS "standard multiplayer FPS". Both claims are refuted for the local skirmish path. The 45 Hz number is not found in the local skirmish code path. Replace with: "Default local skirmish throttle is 1 radar-bucket (~16 ms) per frame = nominal ~62.5 Hz at game speed 1."

---

## Sources

- Ghidra decompile: `Main_Tick @ 0x0055D360` (this session)
- Ghidra decompile: `FUN_0055E160 @ 0x0055E160` (this session)
- Ghidra decompile: `GetRadarTimer @ 0x006C8C40` (prior session, cited in GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md)
- INI: `ini/rulesmd.ini` `[MultiplayerDialogSettings] GameSpeed=1`
- Existing reports (read, not duplicated): `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`, `VISIBLE_PACE_AUDIT_GHIDRA_REPORT.md`, `DEFAULT_SKIRMISH_FRAME_PACE_EXTENSION_GHIDRA_REPORT.md`, `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`
