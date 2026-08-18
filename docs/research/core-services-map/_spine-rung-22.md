# Spine Rung 22 — "V. Wave-splash driver" (Psychic-wave ripple driver)

Part of the `LogicClass::PerTickUpdate` ordered ladder. This rung is the **logic-side**
driver for the expanding "wave"/psychic-ripple effect spawned by the **Psychic Dominator**
super-weapon and by the map "create wave" trigger action.

- **Driver:** `FUN_0053d310` @ `0x0053d310`
- **Body / per-wave worker:** `Wave_splash_forces` @ `0x0053cbe0` (called per active wave)
- **Body site in spine:** `LogicClassPerTickUpdateLiveVector` @ `0x0055afb0`, call at `0x0055b64b`
- **Gate global:** `DAT_00aa0128` (wave-list count)

---

## 1. Purpose (one line)

Per tick, walks every active "wave" entry and advances its expanding psychic-ripple
animation: spawns ripple `AnimClass` sprites, applies area damage at the epicentre, and
writes a per-frame screen-space jitter/shake offset onto every TechnoClass object caught
inside the wave radius; expires the wave after 78 frames.

Verified via `decompile_function 0x0053d310`, `decompile_function 0x0053cbe0`.

---

## 2. What it walks / does

### Driver `FUN_0053d310` (verified via `decompile_function 0x0053d310`)
```c
void FUN_0053d310(void) {
  int iVar1 = DAT_00aa0128;                    // wave-list count
  while (iVar1 = iVar1 + -1, -1 < iVar1) {
    Wave_splash_forces();                      // __fastcall, ECX = current wave entry
  }
}
```
A plain reverse count-down loop over `DAT_00aa0128` active waves. (Note the decomp drops the
ECX argument; each iteration passes the current wave-entry pointer in ECX — confirmed because
`Wave_splash_forces` is `__fastcall(float *param_1)` and dereferences it immediately.)

### Per-wave worker `Wave_splash_forces` @ `0x0053cbe0` (verified via `decompile_function 0x0053cbe0`)
Wave entry layout (floats / reinterpreted ints):
- `param_1[0..2]` = epicentre coord (X, Y, Z)
- `param_1[3]` = **frame counter** (int reinterpreted as float)
- `param_1[4]` = **mode flag** (`== 1.4013e-45`, i.e. the float bit-pattern of integer `1`)

Control flow:
1. **Expiry:** if `param_1[3] > 0x4e` (78) → free the wave, remove its slot via
   `FUN_0053dda0`, `FUN_007c8b3d(param_1)`, return.
2. **Dominator-mode skip:** if `param_1[4] == 1` → jump straight to `LAB_0053d302`
   (just increments `param_1[3]` and returns). The Psychic Dominator marks its wave with
   `+0x10 = 1` (see §6) so the driver does NOT run the ripple/damage loop for it — the
   Dominator does its own mind-control/damage; its wave entry is a pure visual/timer placeholder.
3. **Frame-0 epicentre burst** (`param_1[3] == 0.0`): chooses a splash `AnimClass` type
   (`Rules+0xbc4`/`+0xbd0` indexed entry on land-cell type 2, else `Rules+0x298`), constructs
   it via `AnimClass__Constructor`, then a second anim from `Rules+0x29c`; if the cell has the
   water flag (`cell+0x140 & 0x100`) applies area damage with `Rules+0xff0` warhead at an
   offset depth; always applies a second `Apply_area_damage(0, Rules+0xff0, 1, 0)` and calls
   the shake driver `FUN_0048a620`.
4. **Ripple ring loop** (every frame): nested `-3..+3` cell scan around the epicentre cell
   (`local_6c`/`sStack_6a`). For each occupant in each cell whose RTTI type is `0xf`
   (VoxelAnim-ish) or `1` (Unit/Foot), it: computes screen distance, and if within radius
   (`Math__ftol() + 8 < 0x100`) writes per-object jitter into `piVar2[0xca]`/`piVar2[0xcb]`
   using `Cos_lookup`/`Sin_lookup`/`Sqrt_Approx` math driven by `RateTimer__Current()` and the
   wave frame index `param_1[3]`. This is the visible "objects ripple/shake as the wave passes".
5. **Tick:** `param_1[3] += 1`.

### Companion (render-side, NOT this rung)
`FUN_0053d850` @ `0x0053d850` also loops `DAT_00aa0128` waves but uses `g_PrimarySurface`
and draws via `FUN_0053d580`; its only caller is `TacticalClass_Draw @ 0x006d3d10`
(verified via `get_function_callers 0x0053d850`). That is the render half — out of scope
for the logic spine.

---

## 3. Gate / mode condition (CONFIRMED + clarified)

**The call site is UNCONDITIONAL.** Verified via `disassemble_function 0x0055afb0`:
```
0055b64b: CALL 0x0053d310     ; wave driver — no preceding branch/test
```
It sits immediately after the AnimClass / extra object-vector ticks (loop over `[0x00a80238]`
ending at `0055b6b3`) and immediately before `0055b650: CALL 0x00420e90`
(AlphaShapeClass::PurgeDisabled, rung W). This confirms ORDER 22.

The `DAT_00aa0128 > 0` "gate" listed in the ladder is **internal** to the driver's loop
counter, not a branch at the call site. When `DAT_00aa0128 == 0` the loop body never runs
(the count-down `-1 < iVar1` immediately fails), so the driver is a cheap no-op with zero
waves active. **Verdict: gate is internal-count, call is unconditional — ladder description
is correct in spirit (effectively no-op when count 0) but the call itself has no mode gate.**

---

## 4. RNG draws — which stream, how many, for what

**The driver itself draws NO RNG directly.** `get_function_callees 0x0053cbe0` returns:
`AnimClass__Constructor, Apply_area_damage, Cos_lookup, FUN_0048a620, FUN_0053d8e0,
FUN_0053dda0, FUN_007c8b3d, MapClass__Get_CellClass(_At_Coord), Math__ftol,
RateTimer__Current, Sin_lookup, Sqrt_Approx, TacticalClass__CoordsToClient2, operator_new`
— **no `Random__RandomRanged` / RNG entry point.** The wave motion is fully deterministic
(driven by `RateTimer__Current()` tick value and the frame index, via Cos/Sin/Sqrt lookups).

**Indirect RNG is possible** through the wave's two `Apply_area_damage` calls
(`Apply_area_damage @ 0x00489280`), which DO contain `Random__RandomRanged @ 0x0065c7e0`.
The RNG **instance/stream is Scen->Random**: every RNG call site in `Apply_area_damage`
loads `ECX = [0x00a8b230] + 0x218` (verified via `disassemble_function 0x00489280`, e.g.
`00489fe6: MOV EDX,[0x00a8b230]; 00489fef: LEA ECX,[EDX + 0x218]; 00489ff5: CALL 0x0065c7e0`,
and identically at `0x0048a173`, `0x0048a239`, `0x0048a299`, `0x0048a38e`, `0x0048a3dd`).
`[0x00a8b230]` is `g_ScenarioClass_Instance`; `+0x218` is the embedded **Scen->Random**
generator (the lockstep-synced game RNG) — NOT `g_MainRng`, NOT `g_MapGenRng`.

Which of those RNG branches the WAVE actually reaches:
- **Bridge / wall destruction probability rolls** (`Random__RandomRanged(1, Rules+0x1740)` at
  `0x0048a173`/`0x0048a239`/`0x0048a299`): **bypassed** for the wave. These are skipped when
  the warhead `== Rules+0xff0` (`0048a15d: CMP ECX,[EAX+0xff0]; JZ` jumps past the RNG call),
  and the wave passes exactly `Rules+0xff0` as its warhead. So no draw here.
- **Ore/overlay-destruction debris rolls** (`Random__RandomRanged(0,99)` for VoxelAnim at
  `0x0048a38e` and for the particle system at `0x0048a3dd`): **NOT** warhead-gated; they fire
  whenever the wave's area damage destroys a tiberium/ore overlay on a hit cell. Each
  destroyed overlay consumes up to `Rules+0x68` debris VoxelAnim rolls (loop, breaks on first
  hit < 0xf) plus 1 particle roll. So the wave **can** draw Scen->Random, but only
  conditionally (overlay destroyed on a hit cell — uncommon in practice; the wave warhead's
  damage and target set are small).

**Draw count for the rung:** 0 in the common case (no overlay destroyed). When the wave's
epicentre/ring damage destroys an ore overlay: 1..(Rules+0x68) Scen->Random draws per
destroyed overlay (debris) + 1 (particle), all on Scen->Random, in cell-traversal order
within `Apply_area_damage`.

> Lockstep note: because the only RNG path is Scen->Random and is data-driven by whether the
> wave warhead destroys overlays, the Rust port must reproduce (a) the bypass of bridge/wall
> rolls for the `Rules+0xff0` warhead, and (b) the exact debris-roll loop order when an
> overlay is destroyed, to keep the Scen->Random stream byte-aligned.

---

## 5. Active-in-YR? / Tiberian Sun legacy?

**ACTIVE in standard YR — not TS-dead.** The wave list `DAT_00aa0128` is populated only by
two callers of the spawner `FUN_0053cb10` (verified via `get_function_callers 0x0053cb10`):

1. **`PsychicDominator__MindControlArea` @ `0x0053b080`** — the Psychic Dominator super-weapon
   detonation. This is a core YR superweapon, present and buildable in standard YR skirmish.
   It spawns one wave entry and immediately sets its mode `+0x10 = 1` (Dominator placeholder).
2. **`TriggerAction__Execute` @ `0x006dd8b0`, case `0x5e`** — the map "create wave at waypoint"
   trigger action; spawns a `param_1[4]==0` wave (the full ripple/damage variant). Map-scripted
   (campaign / custom maps).

So in a normal YR skirmish the rung is reachable and player-visible whenever a Psychic
Dominator fires (the dominator's ripple placeholder ticks here; the visible mind-control
ripple itself is mostly drawn render-side). The full ripple+damage+jitter variant is reached
when a map trigger creates a wave. **No SpecialFlags gate, no FogOfWar gate, no TS-only path.**

---

## 6. Spawner detail (for completeness)

`FUN_0053cb10` (verified via `decompile_function 0x0053cb10`) initialises a wave entry
(`[0]=X,[1]=Y,[2]=Z,[3]=0,[4]=0`) and appends it to the wave list at `DAT_00aa011c[DAT_00aa0128]`,
incrementing `DAT_00aa0128`, subject to a capacity check against `DAT_00aa0120`/`DAT_00aa012c`.
- Dominator path sets `*(entry+0x10)=1` AFTER spawn (i.e. `param_1[4]=1`) → driver skip mode.
- Trigger path (case 0x5e) leaves `param_1[4]=0` → driver runs full ripple.

`DAT_00aa0118` (the wave-list element/vector type helper) read live as all-zero
(`read_memory 0x00aa0118` → 0) — consistent with no game/scenario loaded in the current
Ghidra session (allocated at scenario init). This does not affect the static control-flow
findings above.

---

## 7. Confidence

- Driver loop + gate (call unconditional, count-internal): **HIGH** — read from
  `disassemble_function 0x0055afb0` (`0055b64b` has no preceding test) and
  `decompile_function 0x0053d310`.
- No direct RNG in driver: **HIGH** — `get_function_callees 0x0053cbe0` (no RNG callee).
- Indirect RNG = Scen->Random via `Apply_area_damage`, bridge/wall rolls bypassed for the
  wave warhead, debris rolls reachable: **HIGH** — `disassemble_function 0x00489280`
  (ECX = `[0x00a8b230]+0x218` at every `0x0065c7e0` call; `0048a15d` warhead-equality skip).
- Active-in-YR (Dominator + trigger spawners, no TS gate): **HIGH** —
  `get_function_callers 0x0053cb10`, `decompile_function 0x0053b080`,
  `decompile_function 0x006dd8b0` (case 0x5e).
