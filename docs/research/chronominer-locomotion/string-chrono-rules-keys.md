# Chrono Rules INI Keys — bundle decode

**Kind:** string / INI-key bundle
**Task:** decode-string-chrono-rules-keys
**Active in YR:** Yes — all 5 keys are consumed by the teleport locomotor's warp-delay
formula and are present with non-default values in the stock `rulesmd.ini`

---

## Summary

Five `[General]` INI keys control the teleport locomotor's warp-delay timing formula.
They are read by the RulesClass ReadINI path and stored as direct fields in the
`g_RulesClass_Instance` singleton. Four are consumed in `InitiateWarp` (0x00719400)
and one (`ChronoDelay`) is consumed in `StateMachineTick` state 3 (0x007192F0).

All 5 string addresses and Rules struct offsets verified via:
- `inspect_memory_content` on each string address (strings confirmed)
- `decompile_function 0x00719400` (InitiateWarp — Rules+0xBF4/+0xBF8/+0xBFC/+0xC00)
- `decompile_function 0x007192F0` (StateMachineTick state 3 — Rules+0xBEC)

---

## Active in YR

**Yes.** All keys loaded unconditionally from `[General]` section on game startup.
`ChronoTrigger=yes` (default in stock rulesmd.ini) activates the distance-proportional
delay formula. `ChronoDelay` provides the base warp-out hold time used for all teleport
locomotor units including the chrono miner.

---

## Key table

| INI key | String address | String (verified) | Rules offset | Type | Stock value | Role |
|---|---|---|---|---|---|---|
| `ChronoDelay` | `0x0083C714` | "ChronoDelay" | `+0xBEC` | int (frames) | 60 | Base warp-hold delay; written to TechnoClass+0x284 in state 3 |
| `ChronoTrigger` | `0x0083C6D8` | "ChronoTrigger" | `+0xBF8` | bool (char) | yes (1) | If true: scale delay by distance; if false: constant ChronoMinimumDelay |
| `ChronoDistanceFactor` | `0x0083C6E8` | "ChronoDistanceFactor" | `+0xBF4` | int | 48 | Divisor: `timer_ticks = distance / ChronoDistanceFactor` |
| `ChronoMinimumDelay` | `0x0083C6C4` | "ChronoMinimumDelay" | `+0xBFC` | int (frames) | 16 | Floor: computed delay clamped to max(computed, ChronoMinimumDelay) |
| `ChronoRangeMinimum` | `0x0083C6B0` | "ChronoRangeMinimum" | `+0xC00` | int (leptons) | 0 | If distance < ChronoRangeMinimum: override delay to ChronoMinimumDelay |

String verification via `inspect_memory_content`:
- 0x0083C714 → "ChronoDelay" (null-terminated at byte 11, char[12])
- 0x0083C6D8 → "ChronoTrigger" (null-terminated at byte 13, char[14])
- 0x0083C6E8 → "ChronoDistanceFactor" (null-terminated at byte 20, char[21])
- 0x0083C6C4 → "ChronoMinimumDelay" (null-terminated at byte 18, char[19])
- 0x0083C6B0 → "ChronoRangeMinimum" (null-terminated at byte 18, char[19])

INI values from `ini/rulesmd.ini` lines 221–227:
```
ChronoDelay=60
ChronoDistanceFactor=48
ChronoTrigger=yes
ChronoMinimumDelay=16
ChronoRangeMinimum=0
```

---

## Delay formula (verified from InitiateWarp decompile)

Verified via `decompile_function 0x00719400`. `param_1` is `int*` (unaff_ESI).

```c
// InitiateWarp delay computation:

distance = Sqrt_Approx(dx*dx + dy*dy + dz*dz);  // Euclidean distance in leptons
timer_ticks = 0;

if (*(char*)(g_RulesClass_Instance + 0xBF8) != '\0') {  // ChronoTrigger == true
    factor = *(int*)(g_RulesClass_Instance + 0xBF4);     // ChronoDistanceFactor
    timer_start = g_CurrentFrameCounter;
    timer_ticks = distance / factor;                      // distance-proportional delay
}

// elapsed-time subtraction (residual from prior timer state):
if (timer_start != -1) {
    elapsed = g_CurrentFrameCounter - timer_start;
    if (elapsed < timer_ticks) timer_ticks -= elapsed;
    else timer_ticks = 0;
}

// clamp to minimum:
min_delay = *(int*)(g_RulesClass_Instance + 0xBFC);   // ChronoMinimumDelay
if (timer_ticks <= min_delay) {
    timer_start = g_CurrentFrameCounter;
    timer_ticks = min_delay;
}

// range override: if distance < ChronoRangeMinimum, use ChronoMinimumDelay:
if (distance < *(int*)(g_RulesClass_Instance + 0xC00)) {  // ChronoRangeMinimum
    timer_start = g_CurrentFrameCounter;
    timer_ticks = min_delay;                               // same ChronoMinimumDelay
}
```

Final result: `param_1[0xE] = timer_start`, `param_1[0x10] = timer_ticks` written to
`TeleportLocomotionClass+0x38` (timer_start_frame) and `TeleportLocomotionClass+0x40`
(timer_duration_frames).

**Chrono miner instant-warp branch** (also in InitiateWarp):
```c
// If GetMission() == 1 (mission = ENTER / dock)
//    AND TechnoType+0x6C4 field +0xE0E is set (harvester flag):
iVar3 = (**(code**)(*(int*)unaff_ESI[2] + 0x2c))();  // GetMission via vtable+0x2C
if ((iVar3 == 1) && (*(char*)(*(int*)(unaff_ESI[2] + 0x6c4) + 0xe0e) != '\0')) {
    timer_start = g_CurrentFrameCounter;
    timer_ticks = 0;                  // instant warp: duration = 0 frames
    WarpAnimGate = 0;                 // no warp shimmer for chrono miner
}
```
This forces `timer_ticks = 0` for harvester-type units entering dock mission, producing
instant teleportation. Verified in both InitiateWarp and StateMachineTick decompiles.

---

## Rules struct offsets summary

All offsets verified from `decompile_function 0x00719400` direct reads of
`g_RulesClass_Instance + N`:

| Rules offset | Key | Type | Formula role |
|---|---|---|---|
| `+0xBEC` | ChronoDelay | int (frames) | Written to TechnoClass+0x284 in state 3; used as WarpIn hold timer in state 5 |
| `+0xBF4` | ChronoDistanceFactor | int | Divisor for distance-proportional delay |
| `+0xBF8` | ChronoTrigger | char/bool | Gate for proportional formula; if 0, formula skipped |
| `+0xBFC` | ChronoMinimumDelay | int (frames) | Floor for computed delay; also used as override delay for short-range warps |
| `+0xC00` | ChronoRangeMinimum | int (leptons) | Distance threshold below which delay = ChronoMinimumDelay |

Note: `+0xBEC` is from StateMachineTick state 3 (`*(undefined4*)(param_1[2]+0x284) = *(undefined4*)(g_RulesClass_Instance+0xbec)`), not InitiateWarp. The other four offsets are from InitiateWarp directly.

---

## Proposed Ghidra labels

| Symbol | Address | Proposed name |
|---|---|---|
| 0x0083C714 | string | STR_INIKey_ChronoDelay |
| 0x0083C6D8 | string | STR_INIKey_ChronoTrigger |
| 0x0083C6E8 | string | STR_INIKey_ChronoDistanceFactor |
| 0x0083C6C4 | string | STR_INIKey_ChronoMinimumDelay |
| 0x0083C6B0 | string | STR_INIKey_ChronoRangeMinimum |

---

## Out-of-scope refs

- `RulesClass__ReadINI` (reader for all 5 keys) — general Rules infrastructure; not
  teleport-locomotion specific. Address not traced in this decode.
- `Sqrt_Approx` — general math utility; not teleport-specific
- `Math__ftol` — general float-to-long conversion; not teleport-specific

---

## Unverified / YELLOW

- **`ChronoDelay` offset `+0xBEC` from StateMachineTick**: Read as `*(undefined4*)(g_RulesClass_Instance+0xbec)` in state 3 of StateMachineTick. The assignment to `TechnoClass+0x284` is confirmed. Whether this is `ChronoDelay` (vs another field) is inferred from rulesmd.ini comment "delay after teleport for chrono sphere" and the value 60 matching the formula — MEDIUM confidence. Full RulesClass struct decode would confirm definitively.

- **`TechnoType+0x6C4` field at `+0xE0E`**: The `IsHarvester`/`Harvester` flag used for instant-warp branch. `TechnoType+0x6C4` is confirmed as the TechnoType ptr from TechnoClass (the decompile reads `*(int*)(unaff_ESI[2] + 0x6c4)` to get the type). The `+0xE0E` sub-offset on TechnoType is not independently decoded. YELLOW on exact flag name; HIGH on the instant-warp behavior (timer_ticks=0 when set).

- **Reader function address for all 5 keys**: Not traced to a specific RulesClass ReadINI address in this session. The manifest cites `0x00713fe9` as the reader for the Teleporter key; the Chrono* keys are likely read by the same or adjacent RulesClass ReadINI dispatch. YELLOW — not verified.
