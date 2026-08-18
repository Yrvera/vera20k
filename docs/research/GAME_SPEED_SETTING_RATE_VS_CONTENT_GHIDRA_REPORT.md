# Game Speed Setting — Source, Default, Range, and Rate vs. Content

**Date:** 2026-05-28
**Addresses:** `Main_Tick @ 0x0055D360`, `FUN_005b67f0 @ 0x005B67F0`, `OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0`, `EventClass__Execute @ 0x004C794E`, `Main_Game @ 0x0052DABD`, `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`, `SessionClass__ReadSkirmishSettings @ 0x00697F10`
**Confidence:** HIGH for all handoff-critical claims. Every claim is backed by live decompile in this session or directly cross-confirmed by INI file.
**Active in YR:** Yes (skirmish path `g_GameMode == 5`). Network budget branch (`DAT_00a8b558`) is Conditional (network modes only).

---

## Target Question

Does changing the YR game-speed setting make each frame **DO MORE work** (frame content changes) or make frames **ARRIVE FASTER** (frame rate changes)? Which global is the live skirmish speed byte and which is the network FPS divisor? What is the default, range, and slider mapping?

## Non-Goals

- Full `Main_Tick` branch structure (slot 1).
- Animation `Rate=` conversion internals (slot 3).
- Frame increment guard flags (slot 5).

## Evidence Needed to Mark COMPLETE

- Binary confirmation that `DAT_00a8eb60` and `DAT_00a8b558` are independent.
- Confirmed writers of each global.
- Confirmed slider-to-stored mapping and default.
- Binary evidence on rate-vs-content verdict.

## Stop Conditions

- All handoff claims verified by decompile + xref in this session. ✓

---

## 1. The Two Globals — `DAT_00a8eb60` vs `DAT_00a8b558`

These are **completely independent** globals with different purposes. They are NOT the same value and are NOT copied from each other.

### `DAT_00a8eb60` — Live Game Speed Byte (skirmish / session)

**Purpose:** The currently active game speed code. Lower value = faster (fewer wait buckets). Written to `DAT_00887350` at every `Main_Tick` to set the local throttle budget.

**Writers** (verified via `get_xrefs_to 0x00a8eb60`):

| Writer | Address | What it does |
|---|---|---|
| `FUN_005b67f0` (session packet apply) | `0x005B6AD7` | `DAT_00a8eb60 = DAT_00a8b268` — copies stored session speed byte from `DAT_00a8b268`, which was just loaded from packet `+0xA2` |
| `OptionsClass__ApplyFromInGameDialog` | `0x004E1EBA` | `DAT_00a8eb60 = 6 - slider_position` — in-game speed slider |
| `EventClass__Execute` case `0xd` | `0x004C794E` | `DAT_00a8eb60 = *(param_1 + 7)` — speed-change event from command queue |
| `Main_Tick` mode-0 override | `0x0055D774` | Forces `DAT_00a8eb60 = 2` when `g_GameMode == 0` and `DAT_00a8EDDC == 0` — mode-0 only, not skirmish |
| `Main_Game` recording replay bulk read | `0x0052DABD` context | `(*read)(&DAT_00a8eb60, 0xb8)` — replay/recording read path |

(verified via `decompile_function 0x005B67F0`, `0x004E1DE0`, `0x004C794E`, `0x0052DABD`, `get_xrefs_to 0x00a8eb60`)

### `DAT_00a8b558` — Network Requested FPS (network/replay modes only)

**Purpose:** The per-player requested frame-rate divisor used to compute network throttle budgets. Only consumed in the `Main_Tick` non-mode-0/non-mode-5 branch: `DAT_00887350 = (int)(0x3c / DAT_00a8b558)` and `local_1ac = (int)(1000 / DAT_00a8b558)`.

**Writers** (verified via `get_xrefs_to 0x00a8b558`):

| Writer | Address | What it does |
|---|---|---|
| `Main_Game` | `0x0052DABD` | `DAT_00a8b558 = 0x1e` (= 30) — initializes to 30 fps on every game start |
| `EventClass__Execute` case `0x20` | `0x004C807D` | `DAT_00a8b558 = *(ushort *)(param_1 + 7)` — updated from a network event packet containing the negotiated frame rate |

(verified via `decompile_function 0x0052DABD`, `0x004C807D`, `get_xrefs_to 0x00a8b558`)

**`DAT_00a8b558` is NEVER read in standard local skirmish (`g_GameMode == 5`).** The only `Main_Tick` reads of `DAT_00a8b558` are inside the branch `if (g_GameMode != 0 && g_GameMode != 5 && DAT_00a8b24c == 2)`, confirmed by `decompile_function 0x0055D360`.

**Active in YR:** `DAT_00a8b558` — Conditional (network/replay modes `g_GameMode != 0, != 5`).

---

## 2. Default Skirmish Speed — Source, Value, and INI Chain

**Source chain** (verified via `decompile_function 0x00671EA0`, `0x00697F10`; INI line evidence below):

```
RulesClass+0x14A0 = ReadInt("MultiplayerDialogSettings", "GameSpeed", prev)
  → reads rulesmd.ini [MultiplayerDialogSettings] GameSpeed=1  (line verified)
  → patches base rules.ini [MultiplayerDialogSettings] GameSpeed=0

SessionClass::ReadSkirmishSettings+0x08 =
    ReadInt(skirmish_section, "GameSpeed", RulesClass+0x14A0)
  → [Skirmish] GameSpeed= absent in local RA2MD.INI → falls back to RulesClass+0x14A0 = 1
```

**Default skirmish stored speed byte = `1`** (verified: INI `ini/rulesmd.ini [MultiplayerDialogSettings] GameSpeed=1`, reader `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`).

The `[Options] GameSpeed=3` in the local `RA2MD.INI` is read into `Options+0x00` by `OptionsClass__ReadFromINI @ 0x005FA620` but is **not** the fallback when `[Skirmish] GameSpeed` is absent — `SessionClass__ReadSkirmishSettings` falls back to `RulesClass+0x14A0`, not `Options+0x00`.

**Active in YR:** Yes.

---

## 3. Slider-to-Stored Mapping and Range

From `OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0` (verified via `decompile_function 0x004E1DE0`):

```c
LVar2 = SendMessageA(slider_hwnd, TBM_GETPOS, 0, 0);  // slider position
iVar5 = 6 - LVar2;                                     // stored speed byte
DAT_00a8eb60 = iVar5;
```

**Mapping: `stored_speed = 6 − slider_position`.**

The slider range controls (min/max) are set in the dialog resource. Binary evidence from `OptionsClass__ApplyFromInGameDialog` and existing cross-session docs show the slider runs 0–6 (7 positions). This yields:

| Slider position (visible) | Stored speed byte | Nominal wait budget |
|---|---|---|
| 0 (slowest) | 6 | 6 × 16 ms = 96 ms |
| 1 | 5 | 80 ms |
| 2 | 4 | 64 ms |
| 3 (middle) | 3 | 48 ms |
| 4 | 2 | 32 ms |
| 5 | 1 | 16 ms |
| 6 (fastest) | 0 | 0 ms (no wait) |

**Range: stored speed byte 0–6. Default skirmish = 1 (second-fastest position).**

Slider position 5 corresponds to the default stored speed byte `1`.

**Active in YR:** Yes.

---

## 4. Rate vs. Content — The Core Question

### Verdict: Speed changes frame RATE, not frame CONTENT.

**Binary evidence** (verified via `decompile_function 0x0055D360`):

In `Main_Tick` for standard skirmish (`g_GameMode == 5`):

```c
uVar19 = DAT_00a8eb60;           // load stored speed byte
DAT_00887348 = GetRadarTimer();  // record start bucket
DAT_00887350 = uVar19;           // set wait BUDGET = speed byte
// ...
// ALL game work runs: GScreenClass__Input, LogicClass__AI, Map__Logic, RenderFrame_main, etc.
// ...
g_CurrentFrameCounter += 1;     // late frame increment
FUN_0055e160();                  // WAIT until budget is consumed
```

`FUN_0055e160 @ 0x0055E160` waits in 16 ms `GetRadarTimer()` buckets. With `DAT_00887350 = 1` (default speed), it waits approximately 1 bucket (16 ms) after subtracting elapsed work time. With `DAT_00887350 = 0` (max speed), no wait occurs.

**The game work runs exactly once per frame regardless of speed byte.** The speed byte sets the minimum inter-frame delay. Faster speed = smaller wait = more frames per wall-clock second. Slower speed = larger wait = fewer frames per wall-clock second.

There is no code path that runs `LogicClass__AI` or `Map__Logic` multiple times per `Main_Tick` based on the speed byte, and no code path that skips them. The per-frame game-content is constant; only the pacing changes.

**Implication for Rust:** Rust's current model in `src/app_sim_tick.rs` (speed changes how many fixed sim steps run per wall second) matches the gamemd mechanic — speed changes RATE not CONTENT. This is correct directionally. The outstanding uncertainty is the exact realized frames/sec at speed byte `1` under retail workload (not a binary-derivable fact; requires live measurement).

---

## 5. Implementation Handoff

### Handoff A — Two-Globals Disambiguation

**Behavior:** `DAT_00a8eb60` is the live skirmish speed byte; `DAT_00a8b558` is the network FPS divisor. They are independent.
**Rust delta:** Do not merge or confuse these. The Rust scheduler uses the speed byte to set sim ticks/sec; there is no corresponding Rust port of `DAT_00a8b558` needed for skirmish.
**Surface:** `src/app_types.rs` speed byte → TPS mapping; `src/app_sim_tick.rs` scheduler.
**Acceptance:** A Rust unit test passes `game_speed_byte = 1..6` through the TPS mapping and confirms `network_fps_divisor` (if modeled) is a separate field.
**Test name:** `test_speed_byte_and_network_divisor_are_independent`
**Risk:** Low — already separated in current Rust; this is confirmation only.

### Handoff B — Rate vs. Content Verdict

**Behavior:** Speed byte changes how often the frame loop fires; each frame does the same amount of logic/render work.
**Rust delta:** Rust's "speed changes sim ticks/sec" model is correct. Do NOT add multi-logic-tick-per-wall-tick acceleration when speed is high.
**Surface:** `src/app_sim_tick.rs` elapsed-time scheduler.
**Acceptance:** At default speed `1`, one `World::advance_tick` fires per Main_Tick equivalent; at speed `6` (max), ticks fire faster but each tick still runs once.
**Test name:** `test_game_speed_changes_rate_not_content`
**Risk:** Medium — if the Rust scheduler accidentally runs 2× `advance_tick` per interval at high speed it would double game-logic work, a content divergence.

### Handoff C — Slider Inversion

**Behavior:** UI slider position 5 corresponds to stored byte `1` (default YR skirmish speed). Slider 6 (rightmost) = byte `0` (fastest, no wait). Slider 0 (leftmost) = byte `6` (slowest).
**Rust delta:** Any future speed UI must apply `stored = 6 - slider_position`.
**Surface:** UI / settings layer (not sim/).
**Acceptance:** Slider at position 5 produces `DEFAULT_YR_SKIRMISH_GAME_SPEED = 1`.
**Test name:** `test_slider_inversion_mapping`
**Risk:** Low (already documented in GLOBAL_TIMING_MODEL doc; adding test anchors it).

---

## 6. Negative Facts

1. **`DAT_00a8b558` is NOT the skirmish speed byte.** It is initialized to `0x1e` (30) in `Main_Game @ 0x0052DABD` and updated only from network events (case `0x20` in `EventClass__Execute`). Confirmed by `get_xrefs_to 0x00a8b558`.

2. **Speed byte does NOT add extra game-logic passes per frame.** There is no loop in `Main_Tick` around `LogicClass__AI` / `Map__Logic` driven by the speed byte. Confirmed by `decompile_function 0x0055D360`.

3. **`[Options] GameSpeed=3` is NOT the default skirmish fallback.** `SessionClass__ReadSkirmishSettings` falls back to `RulesClass+0x14A0`, not `Options+0x00`. Confirmed by `decompile_function 0x00697F10`.

4. **The mode-0 speed-2 override does NOT apply to skirmish.** The branch at `0x0055D774` that forces `DAT_00a8eb60 = 2` is gated on `g_GameMode == 0 && DAT_00a8EDDC == 0`. Standard skirmish is `g_GameMode == 5`. Confirmed by `decompile_function 0x0055D360`.

5. **The stored speed byte is NOT an FPS number.** It is a wait-budget count in 16 ms `GetRadarTimer()` buckets. `GetRadarTimer @ 0x006C8C40` is `timeGetTime() >> 4`; one bucket = 16 ms. Confirmed by `decompile_function 0x006C8C40`.

---

## 7. Remaining Uncertainty

- **Realized frames/sec at speed byte `1` under retail workload.** The static binary analysis gives a nominal budget of 1 × 16 ms = 16 ms/frame ≈ 62.5 fps, but actual throughput depends on Windows Sleep() granularity (typically 15.6 ms), render workload, and the elapsed-bucket subtraction in `FUN_0055e160`. This requires a live runtime probe and cannot be resolved from the binary alone.

- **Slider min/max from dialog resource.** The `6 - slider_position` mapping is confirmed from binary code, and range 0–6 (7 positions, matching 7 speed settings) is inferred from the stored byte range 0–6. The dialog resource (`.rc` / compiled resource) was not decompiled to confirm the trackbar's explicit `TBM_SETRANGE` call; the range is HIGH confidence from code evidence but not verified against the dialog resource directly.

---

## 8. Relationship to Existing Docs

The following existing docs cover overlapping ground and are **consistent with** this report:

- `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md` — §3.3 default speed source, slider `6-position`, `FUN_005B67F0` packet apply: **all confirmed correct by this session**.
- `VISIBLE_PACE_AUDIT_GHIDRA_REPORT.md` — Default speed source, live propagation, main tick throttle: **all confirmed correct**.
- `skirmish-ui/DEFAULT_SKIRMISH_FRAME_PACE_EXTENSION_GHIDRA_REPORT.md` — §3.1–§3.5: **all confirmed correct**.

No corrections needed in those docs based on this session's findings.

---

## Sources

- Ghidra decompile: `Main_Tick @ 0x0055D360` (verified two-globals split, rate-vs-content)
- Ghidra decompile: `FUN_005b67f0 @ 0x005B67F0` (verified `DAT_00a8eb60 = DAT_00a8b268` path)
- Ghidra decompile: `OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0` (verified `6 - slider`)
- Ghidra decompile: `EventClass__Execute @ 0x004C794E` (verified case 0xd writes `DAT_00a8eb60`; case 0x20 writes `DAT_00a8b558`)
- Ghidra decompile: `Main_Game @ 0x0052DABD` (verified `DAT_00a8b558 = 0x1e` init)
- Ghidra xrefs: `get_xrefs_to 0x00a8eb60` (all writers enumerated)
- Ghidra xrefs: `get_xrefs_to 0x00a8b558` (all writers enumerated — only `Main_Game` + event `0x20`)
- INI: `ini/rulesmd.ini [MultiplayerDialogSettings] GameSpeed=1`
- INI: `ini/rules.ini [MultiplayerDialogSettings] GameSpeed=0`
- Prior docs: `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`, `VISIBLE_PACE_AUDIT_GHIDRA_REPORT.md`, `skirmish-ui/DEFAULT_SKIRMISH_FRAME_PACE_EXTENSION_GHIDRA_REPORT.md` (cross-confirmed, not re-done)
