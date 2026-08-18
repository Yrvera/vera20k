# Spine Rung 16 — P. LightningStorm / PsychicDominator process (`LightningStorm__Process`)

Driver: `0x0053A6C0` (`LightningStorm__Process`)
Body call site: `0x0055b5c8` inside `LogicClass::PerTickUpdate` (`LogicClassPerTickUpdateLiveVector` @ `0x0055AFB0`), order #16.
Single caller — verified via `get_xrefs_to 0x0053A6C0` → `From 0055b5c8 in LogicClassPerTickUpdateLiveVector [UNCONDITIONAL_CALL]`.

## Verdict summary

| Field | Value |
|---|---|
| Order | 16 (after rung O `LaserDrawClass__UpdateAllAI` @ `0x00550150`; before rung Q EMP-pulse reverse loop `DAT_00b04bd4`) |
| Gate | **Unconditional call** from the spine (no caller-side gate). Internally state-gated by `DAT_00a9fabc` (PD warp/fade timer) and `DAT_00a9fab4`/`DAT_00a9fad0` (storm active / cleanup). Idle when no storm and no PD warp active. |
| Draws RNG | **Yes** — only in the scatter-bolt sub-branch |
| RNG stream | **Scen->Random** (`Scenario+0x218`) — receiver is `*(g_ScenarioClass_Instance) + 0x218` |
| RNG draws | 2 per scatter attempt (X offset, then Y offset); up to 3 attempts per qualifying tick |
| Active in YR | **Yes** — Weather/Lightning Storm SW (Soviet) and Psychic Dominator (Yuri) are stock YR superweapons |
| TS legacy | No (the Ion-cpp string is TS-era source filename, but the system is live YR) |

## Purpose (one line)

Per-tick driver for the in-progress Lightning/Weather Storm superweapon (spawns
cloud bolts + ground strikes, manages duration/lighting/EVA), and — in the same
function body — the Psychic Dominator screen-warp / mind-control-area state machine.

## What it walks / does

Verified via `decompile_function 0x0053A6C0` and `disassemble_function 0x0053A6C0`.

1. **PD lighting timer transitions** (top of function): `DAT_00a9fabc == 1 → 2` and
   `== 2 → 0` gated on `DAT_00827fcc != -1 && DAT_00827fc8 + DAT_00827fcc < g_CurrentFrameCounter`.
   On the 1→2 edge it calls `FUN_0053c280` (SuperWeaponEffects lighting update) and
   `FUN_004f42f0(1)` (sidebar/flag redraw). No RNG.
2. **`PsychicDominator__Process()`** (`0x0053af40`) — PD warp/mind-control state machine
   (cases 1..5; calls `PsychicDominator__MindControlArea`). **No RNG** (verified via
   `decompile_function 0x0053af40`).
3. **`Process_QueuedEvents()`** (`0x0053b560`) — storm intro/cloud-fade + EVA/sound
   state machine (cases 1..3; palette fade, `RenderFrame_main`, Voc/Vox playback).
   **No RNG** (verified via `decompile_function 0x0053b560`).
4. Three bolt-array reverse walks (anim half-life / end-of-life purge with in-place
   compaction):
   - `DAT_00a9fa1c` / count `DAT_00a9fa28` — cloud-bolt list; purge when anim age
     `[+0xac] >= totalFrames/2`.
   - `DAT_00a9fa64` / count `DAT_00a9fa70` — strike-pending list; when anim age
     `> totalFrames/2`, call `LightningStorm__GroundStrike(coords...)` (vtable +0x48
     coord fetch), then remove.
   - `DAT_00a9f9d4` / count `DAT_00a9f9e0` — active cloud list; purge when anim age
     `>= totalFrames-1`. When this list drains to 0 and `DAT_00a9fab4` was set, it
     resets the storm and calls `FUN_0053c280`.
   None of these three walks draw RNG.
5. **Spawn branch** (only when storm active `DAT_00a9fab4 != 0 && DAT_00a9fad0 == 0`):
   - Duration expiry check (`DAT_00827fc4`/`DAT_00827fc0`) → set cleanup flag, return.
   - `g_CurrentFrameCounter % Rules+0x17a0 == 0` → `LightningStorm__CreateCloudBolt`
     at storm center `DAT_00a9f9cc`. (center cloud, **no RNG**).
   - `g_CurrentFrameCounter % Rules+0x17a4 == 0` → **scatter loop** (the RNG site).
6. **Cleanup branch** (`DAT_00a9fab4 == 0 || DAT_00a9fad0 != 0`): decrement
   `DAT_00a9fab8` (deferment countdown); on reaching 0 call `LightningStorm__Start`;
   every `0xe1` (225) frames during deferment, optional EVA + on-screen "incoming"
   message (`FUN_005d3ba0`, gated by `Rules+0x17b0`). **No RNG.**

## RNG draws — Scen->Random, 2 per attempt, up to 3 attempts

Verified site in disassembly (`disassemble_function 0x0053a6c0`):

```
0053a994: MOV ECX,dword ptr [0x00a8b230]      ; ECX = g_ScenarioClass_Instance
0053a99a: MOV EAX,[0x00a9f9cc]                ; storm center
0053a99f: PUSH EBP                            ; +spread/2
0053a9a0: PUSH EBX                            ; -spread/2   (EBX = -(Rules+0x17a8 >> 1))
0053a9a1: ADD ECX,0x218                       ; ECX = Scenario+0x218  (Scen->Random)
0053a9ab: CALL 0x0065c7e0                     ; Random__RandomRanged  -> X offset
0053a9b6: ADD word ptr [ESP + 0x14],AX
0053a9bd: LEA ECX,[EDX + 0x218]               ; EDX = g_ScenarioClass_Instance again
0053a9c3: CALL 0x0065c7e0                     ; Random__RandomRanged  -> Y offset
```

- **Function identity:** `0x0065c7e0` is genuinely `Random__RandomRanged`
  (`__thiscall`, walks an RNG state buffer at `param_1 + 0xc + index*4`; verified via
  `decompile_function 0x0065c7e0`). Decompiler label confirmed by body, not taken on faith.
- **Stream identity:** receiver `ECX = *(0x00a8b230) + 0x218`. `0x00a8b230` is
  `g_ScenarioClass_Instance` (verified via `list_globals g_ScenarioClass_Instance` →
  `@ 00a8b230`). The embedded RNG at `Scenario+0x218` is seeded by
  `Init_Random_Number_System` (verified via `decompile_function 0x0052fe1c`:
  `puVar4 = (undefined4 *)(g_ScenarioClass_Instance + 0x218)` then `Random__Seed`).
  This is the deterministic/lockstep **Scen->Random** stream — NOT `g_MainRng`
  (separate symbol `@ 0x00886b88`, seeded in the very next loop of the same function)
  and NOT `g_MapGenRng`.
- **Range:** `RandomRanged(-(CellSpread>>1), +(CellSpread>>1))`. `CellSpread` =
  `Rules+0x17a8` = `LightningCellSpread` (stock 10) → `±5` (`EBP = spread>>1`,
  `EBX = -EBP`).
- **Count:** 2 draws per scatter attempt (X then Y). The attempt loop initializes
  `iStack_10 = 3` (`0053a986: MOV dword ptr [ESP + 0x18],0x3`) and retries on a
  rejected candidate (out of bounds via `Cell_in_bounds_check` `0x00568300`, or within
  `LightningSeparation` = `Rules+0x17ac` manhattan distance of an existing cloud).
  A rejected candidate still costs its 2 draws. So **0, 2, 4, or 6 draws** on a
  qualifying tick (those that pass `g_CurrentFrameCounter % LightningScatterDelay == 0`),
  depending on how quickly a valid cell is found; 0 draws on non-qualifying ticks and
  whenever no storm is active.

This matches the prior verified analysis in
`docs/research/LIGHTNING_STORM_RNG_CLASSIFICATION_GHIDRA_REPORT.md` (sites
`0x0053A9AB`/`0x0053A9C3`) and
`docs/research/LIGHTNING_STORM_SUPERWEAPON_GHIDRA_REPORT.md` §5.

## Gate confirmation (vs spine ladder text)

Spine ladder said: "gate internal storm state `DAT_00a9fabc`; idle when no storm active."

**Correction/refinement:** the call from the spine is **unconditional**. `DAT_00a9fabc`
gates only the **PsychicDominator lighting-fade timer** at the top of the function
(values 1/2), not the lightning bolts. The lightning-bolt list walks are unconditional
reverse loops (empty when counts are 0). The bolt *spawn* branch is gated by storm-active
`DAT_00a9fab4 != 0 && DAT_00a9fad0 == 0`; the RNG-drawing scatter sub-branch is
additionally gated by `g_CurrentFrameCounter % LightningScatterDelay == 0`. "Idle when
no storm active" is observably correct for the bolts, but the function still runs PD and
queued-event state machines every tick. (`get_xrefs_to 0x0053A6C0`,
`disassemble_function 0x0053a6c0`.)

## Rules offsets touched (verified mapping)

| Rules offset | INI key | Stock YR | Use in this driver |
|---|---|---|---|
| `0x17a0` | `LightningHitDelay` | 10 | center cloud-bolt cadence (`% == 0`) |
| `0x17a4` | `LightningScatterDelay` | 5 | scatter-bolt cadence (gate for RNG branch) |
| `0x17a8` | `LightningCellSpread` | 10 | halved → `±5` RNG range |
| `0x17ac` | `LightningSeparation` | 3 | min city-block distance between bolts (reject) |
| `0x17b0` | (deferment-message enable) | — | gates EVA + on-screen incoming text |

(Cross-checked against `ini/rulesmd.ini` lines 134/135/137 and
`docs/research/LIGHTNING_STORM_SUPERWEAPON_GHIDRA_REPORT.md` §3.)

## Active-in-YR / TS-legacy judgment

**Active in YR: Yes.** `LightningStorm__Start` (`0x00539eb0`) is called from
`SuperClass__Launch` (`0x006cc390`), `TriggerAction__Execute` (`0x006dd8b0`), and
`TeamClass__Recruit_Or_Add` (`0x006e9380`) (verified via
`get_function_callers 0x00539eb0`). Weather Storm (`[LightningStormSpecial] Type=LightningStorm`)
is a stock Soviet SW and the Psychic Dominator is the stock Yuri SW; both are reachable
and visible in a normal YR skirmish. The `s_D__ra2mdpost_Ion_cpp_...` source-path string
is a TS-era filename artifact, not evidence of dead code — the code path is live.

**TS legacy: No.**

## Ghidra calls cited

- `decompile_function 0x0053A6C0` — driver body.
- `disassemble_function 0x0053A6C0` — RNG receiver/stream + gate confirmation.
- `decompile_function 0x0055AFB0` — spine body, call site order.
- `get_xrefs_to 0x0053A6C0` — single unconditional caller @ `0x0055b5c8`.
- `decompile_function 0x0065c7e0` — `Random__RandomRanged` identity.
- `read_memory 0x00a8b230` / `list_globals g_ScenarioClass_Instance` / `list_globals g_MainRng` — stream pointer identity.
- `decompile_function 0x0052fe1c` (`Init_Random_Number_System`) — proves `Scenario+0x218` is the Scen->Random buffer, distinct from `g_MainRng`.
- `decompile_function 0x0053af40` / `0x0053b560` — nested PD + queued-event state machines, no RNG.
- `get_function_callers 0x00539eb0` — storm-start reachability (SW launch / trigger / team).
