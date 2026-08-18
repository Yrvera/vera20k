# g_CurrentFrameCounter — decode

**Address:** `0x00a8ed84`  
**Kind:** Global (int32, engine-wide frame counter)  
**Runbook:** global-decode-v1  
**Decoded:** 2026-05-24

---

## Summary

`g_CurrentFrameCounter` is the engine-wide game tick counter. It increments
once per logic tick. The Iron Curtain system uses it as the time base for
elapsed-frame comparisons:

- **Apply:** `TechnoClass__IronCurtain` stores `g_CurrentFrameCounter` at
  `TechnoClass + 0x18c` (the IC apply frame).
- **Query:** `TechnoClass__IsIronCurtainActive` computes
  `elapsed = g_CurrentFrameCounter - apply_frame` and compares against
  `ic_duration`.

**Active in YR: Yes** — shared engine infrastructure, used throughout the game.

**INTERNAL-ONLY from IC perspective.** The specific address and value of this
global are invisible to the player. What is observable is only the IC duration
timing (which depends on this counter's increment frequency = 1 per game tick).

---

## Type and address

| Field | Value |
|-------|-------|
| Address | `0x00a8ed84` |
| Size | 4 bytes (signed int32) |
| Type | `int` |
| Default | `0` (reset at game start by `Main_Game`) |

Verified via `get_xrefs_to 0x00a8ed84` (yields reads and writes) and
`decompile_function Main_Game` (`g_CurrentFrameCounter = 0;` confirmed at startup).

---

## Writers

| Address | Function | When | Value written |
|---------|----------|------|---------------|
| `Main_Game` | `Main_Game` | Game start / scenario reset | `0` |
| `0x0055de81` | `Main_Tick` | Once per logic tick | `g_CurrentFrameCounter + 1` (increment) |
| `0x006846XX` | `ScenarioClass__Read_Scenario` | Save/load scenario | Restore from save |

Verified: `Main_Game` write confirmed via `decompile_function 0x0052da08`. 
`Main_Tick` write at `0x0055de81` confirmed via `get_xrefs_to 0x00a8ed84`.

---

## IC-relevant readers

| Function | Usage |
|----------|-------|
| `TechnoClass__IronCurtain` (`0x0070e2b0`) | Stores current frame as IC apply timestamp at `TechnoClass+0x18c`. |
| `TechnoClass__IsIronCurtainActive` (`0x0041bf40`) | Reads to compute `elapsed = frame - apply_frame`. |
| `TechnoClass__StartFidget` dispatch (`0x004deae4`) | Reads for `param_1[0x1a8] = g_CurrentFrameCounter` (TechnoClass+0x6a0). |

Other readers in the engine (LightningStorm, RateTimer, AnimClass, etc.) are
out of scope for this IC decode.

---

## Out-of-scope refs

All non-IC engine systems that read `g_CurrentFrameCounter` (> 100 sites in the binary per `get_xrefs_to`) are out of scope. The full reader list is an engine-wide concern, not IC-specific.

---

## Unverified (YELLOW)

- The exact increment instruction in `Main_Tick` at `0x0055de81` was not disassembled in this session. The write is confirmed by xref but the specific operation (MOV vs INC) is unverified. Given context (frame counter semantics + zero at start), increment by 1 per tick is the expected and standard behavior.
