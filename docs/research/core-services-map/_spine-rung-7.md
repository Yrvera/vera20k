# Spine Rung 7 — G. IonStorm / weather color interpolation (screen recolor tween)

Verification of LogicClass::PerTickUpdate rung #7. Driver `FUN_004ae4c0` (per-cell color
recompute over all map cells) + `FUN_004f42f0(this=MapClass, 1)` (redraw flag), invoked from
the spine body `LogicClassPerTickUpdateLiveVector` @ `0x0055AFB0`, at the rung-7 gate/body
site `LAB_0055b33d`–`LAB_0055b4d7`. Storm-state gate functions:
`FUN_0053a110` / `FUN_0053a120` / `FUN_0053bad0` / `FUN_0053b400`.

Authority: binary -> Ghidra -> docs. All addresses below cite the verifying Ghidra MCP call.

## Verdict (one line)

Rung 7 is the **global weather/storm screen-recolor tween**: it steps the scenario ambient
color level (`Scen[0xd4b]`) toward its target (`Scen[0xd4c]`) on a timer, then **walks every
map cell and recomputes its per-cell tint/brightness** (`FUN_004ae4c0` → `Cell_ComputeZAdjust`)
so the whole screen fades to/from the storm color. It draws **no RNG**. It is **active in
standard YR** — fired by the Psychic Dominator, Lightning Storm, and Weather (Ion) Storm
superweapon effects — but idle (gate skipped) when no color tween is in progress.

## Position in the ladder

- Runs immediately after rung 6 (`FUN_004acbc0`, FogOfWar second channel) at `LAB_0055b33d`.
- Runs immediately before rung 8 (`TiberiumClass__GrowthDriver_AllTypes`) at `LAB_0055b4d7`.
- Distinct from rungs 4/6 (which are CellIterator shroud/lighting channels): this rung's
  cell walk recomputes the *color tint* fields (`+0x10a/+0x10c/+0x10e`) from the scenario
  storm-color registers, not the shroud edge state.

## Exact gate (CONFIRMED)

From the spine body (verified via `decompile_function 0x0055afb0`), the rung-7 entry at
`LAB_0055b33d`:

```
if (Scen[0xd4c] == Scen[0xd4b]) skip;                 // target color == current color → no tween
if (*(double*)(Rules+0x1668) == 0.0) skip;            // tween-rate rule == 0.0 → disabled
// timer check on Scen[0x492] (last-update frame) vs Scen[0x494] (interval):
//   if not yet elapsed → skip this tick
```

- `g_ScenarioClass_Instance[0xd4c] != g_ScenarioClass_Instance[0xd4b]` — target color word
  must differ from current color word. CONFIRMED.
  (`0xd4b*4 = 0x352c`, `0xd4c*4 = 0x3530` — byte offsets into ScenarioClass.)
- `*(double*)(g_RulesClass_Instance + 0x1668) != 0.0` — CONFIRMED (compared against
  `_g_Const_0_0` = 0.0).
- Timer slots: `Scen[0x492]` (last-update frame, `=0xffffffff` means "not started"),
  `Scen[0x494]` (interval). On elapse the new interval is recomputed via `Math__ftol`
  (`0x007c5f00`) and stored back into `Scen[0x492]/[0x493]/[0x494]`. CONFIRMED via the
  body decompile.

Plan gate `Scen[0xd4c]!=Scen[0xd4b] AND *(double*)(Rules+0x1668)!=0.0 (Scen+0x492 timer)`
is **CONFIRMED exactly**.

## What the body + driver do (CONFIRMED)

Body site `LAB_0055b33d`–`LAB_0055b4d7` (verified via `decompile_function 0x0055afb0`):

1. **Storm-state poll** to pick a step mode: `FUN_0053a110` (storm==1), `FUN_0053a120`
   (storm==2), `FUN_0053bad0` (`DAT_00a9fab0!=0`), `FUN_0053b400` (`DAT_00a9fac0!=0`).
   All four are 1-line state reads (verified `decompile_function` each):
   - `FUN_0053a110` → `DAT_00a9fabc == 1`
   - `FUN_0053a120` → `DAT_00a9fabc == 2`
   - `FUN_0053bad0` → `DAT_00a9fab0 != 0`
   - `FUN_0053b400` → `DAT_00a9fac0 != 0`
   If a storm/effect is active it loads the target color from `Scen[0xd65]`/`Scen[0xd5e]`;
   otherwise it recomputes the interval (`Math__ftol`) and reschedules.
2. **Step the current color** `Scen[0xd4b]` one increment toward `Scen[0xd4c]` (clamped so
   it does not overshoot the target), via the `+= iVar6` / `-= iVar6` branch with `iVar6 =
   Math__ftol()`. Pure integer/FPU arithmetic, no RNG.
3. Set placement-redraw byte `Scen+0x34ab = 1`.
4. **`FUN_004ae4c0()`** — the per-cell recolor driver (below).
5. **`FUN_004f42f0(ECX=0x87f7e8 MapClass, 1)`** — sets the tactical redraw flag
   (`g_Tactical+0xd7d=1`), sets MapClass redraw priority (`[ECX+0xc]`), and calls
   `MapClass__IncrementBridgeCounter` (`0x00578ac0`, +1 to a redraw counter byte). No RNG.
   (Call-site arg decode verified via `read_memory 0x0055b4c0` = `…6a 01 b9 e8 f7 87 00 e8…`
   → `PUSH 1` then `MOV ECX,0x87f7e8` → `FUN_004f42f0(this=0x87f7e8, param2=1)`.)

**`FUN_004ae4c0`** (verified via `decompile_function`/`disassemble_function 0x004ae4c0`),
`ECX = 0x87f7e8` (g_DisplaySingleton / MapClass `this`):
- `CellIterator_Init` (`0x00578350`), then loop `CellIterator_Next` (`0x00578290`) calling
  `Cell_ComputeZAdjust` (`0x00484680`) on every returned cell. Plain grid stepping, no RNG.

**`Cell_ComputeZAdjust`** (verified via `decompile_function 0x00484680`):
- Computes a base level from `Scen+0x352c` (= `Scen[0xd4b]`, the tweened global color level),
  then adds a per-cell tint delta selected by storm state. The branch helpers `FUN_0053a100`
  (`→ DAT_00a9fab4`), `FUN_0053b400`, `FUN_0053a110` choose which scenario tint-register set
  to use (`Scen+0x3540/0x3544` normal, `Scen+0x3558/0x355c`, `Scen+0x3570/0x3574`,
  `Scen+0x358c/0x3590`). Writes per-cell color/brightness fields `+0x10a`, `+0x10c`, `+0x10e`,
  each clamped to `[0, 2000]`. Pure arithmetic, **no RNG**.

So the rung is: advance one color step on the timer, then repaint every cell with the new
tint. This is the screen-wide color flash that accompanies storm/superweapon effects.

## RNG draws

**NONE.** `draws_rng = false`, `rng_stream = none`.

Every callee in the rung is deterministic:
- Gate polls `FUN_0053a110/0053a120/0053bad0/0053b400` and `FUN_0053a100` — single global reads.
- `Math__ftol` (`0x007c5f00`) — FPU float→long round/truncate, verified `decompile_function`.
- `FUN_004ae4c0` cell loop — `CellIterator_Init/_Next` are pure pointer/grid arithmetic
  (verified `decompile_function 0x00578290`); `Cell_ComputeZAdjust` is pure integer math.
- `FUN_004f42f0` → `MapClass__IncrementBridgeCounter` — flag/counter set only.

No `Scen->Random`, `g_MainRng`, or `g_MapGenRng` receiver (ECX) appears on any call in the
chain. The lockstep RNG-draw order is unaffected by this rung whether or not the gate fires.

## Active-in-YR

**YES — conditional, but live in standard YR.** `active_in_yr = conditional` (fires only
while a color tween is in progress: target color ≠ current color and the rule rate ≠ 0).

- The storm-state flags that drive the tween target and the per-cell tint selection are set
  by live YR superweapon effects, NOT TS-only weather:
  - `DAT_00a9fac0` (gate-fn-4 / `FUN_0053b400`) is **WRITTEN by `PsychicDominator__Process`
    and by `FUN_0053ae50`** (the Psychic Dominator activation — gated on Rules+0x2fc/0x300,
    the Dominator anim/blast types) and reset by `SuperWeaponEffects__ResetAll`.
    Verified via `get_xrefs_to 0x00a9fac0` and `decompile_function 0x0053ae50`.
  - `DAT_00a9fabc` (gate-fns 1/2) is the Lightning/Ion-storm state machine state
    (`LightningStorm__Process` family, rung 16). Read 0 at rest (`read_memory 0x00a9fabc`).
- The Psychic Dominator is a **YR-exclusive superweapon**; its screen-recolor flash is the
  canonical player-visible output of this rung. The Lightning Storm (Weather) superweapon is
  also live in YR. So the recolor tween fires in normal YR skirmishes whenever one of those
  effects plays.
- `ts_legacy = false`. The "IonStorm" label is TS terminology, but the *mechanism* (global
  color tween + per-cell repaint) is reused and active for YR superweapon effects. It is NOT
  dead code, unlike rung 6's FogOfWar channel.
- When idle (no tween in progress) the rung is two compares + a timer check with no side
  effects and no RNG, so it does not perturb the lockstep contract.

## Ghidra calls cited

- `decompile_function 0x0055afb0` (spine body, rung-7 site `LAB_0055b33d`–`LAB_0055b4d7`)
- `read_memory 0x0055b4c0` (FUN_004f42f0 call-site arg decode: PUSH 1 + ECX=0x87f7e8)
- `decompile_function 0x004ae4c0`, `disassemble_function 0x004ae4c0` (per-cell recolor driver)
- `decompile_function 0x00484680` (Cell_ComputeZAdjust — per-cell tint math, no RNG)
- `decompile_function 0x00578290` (CellIterator_Next — deterministic grid step)
- `decompile_function 0x0053a110`, `0x0053a120`, `0x0053bad0`, `0x0053b400`, `0x0053a100`
  (storm-state poll helpers — 1-line global reads)
- `decompile_function 0x004f42f0`, `disassemble_function 0x004f42f0` (redraw-flag setter)
- `decompile_function 0x00578ac0` (MapClass__IncrementBridgeCounter — counter +1)
- `decompile_function 0x007c5f00` (Math__ftol — FPU round, no RNG)
- `get_xrefs_to 0x00a9fac0` (writers = PsychicDominator__Process / FUN_0053ae50 / ResetAll)
- `decompile_function 0x0053ae50` (Psychic Dominator activation sets DAT_00a9fac0=1)
- `read_memory 0x00a9fabc` (storm state == 0 at rest)
- `get_function_callers 0x0053a6c0` (LightningStorm__Process called only from the spine)
