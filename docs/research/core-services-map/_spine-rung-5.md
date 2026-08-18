# Spine Rung #5 — E. RecalcBridgeShroudFlags (frame % 120)

**Rung:** order 5 ("E") in `LogicClass::PerTickUpdate` (`LogicClassPerTickUpdateLiveVector`
@ `0x0055AFB0`).
**Driver:** `MapClass__RecalcBridgeShroudFlags` @ `0x00578100`.
**Body site:** `0055b29a`–`0055b2be` inside `0x0055AFB0`.
**Status:** VERIFIED from binary this session.

Verification calls used:
- `decompile_function 0x0055AFB0` (body) + `disassemble_function 0x0055b29a` (the body
  region — disassembly is the ground-truth for the gate).
- `decompile_function 0x00578100` + `disassemble_function 0x00578100` (driver).
- `decompile_function 0x00578290` (CellIterator_Next), `0x006da7d0` (Tactical redraw-queue
  append), `0x005865f0` (CellChangeNotify), `0x006d8700` (Shroud_EdgeBitmask_Calculator).
- `get_function_callers 0x00578100`.
- `decompile_function 0x006e1a70` (the non-per-tick second caller, for context).

---

## Purpose (one line)

Every 120 frames, re-derive the shroud/visibility edge bookkeeping for all **bridge cells**
and queue any changed cells for redraw — keeps the shroud edge rendering around bridges
consistent with ground shroud.

## What it walks / does

`__fastcall`, single arg = `this` = the global `MapClass` instance (ECX = `0x87f7e8`,
confirmed at body site `0055b2a8 MOV ECX,0x87f7e8`). Two sequential full-map cell iterations
using the MapClass cursor window (`+0xf4` map width, `+0x10c/0x110/0x114/0x118` iterator
state) driven by `MapClass__CellIterator_Next` @ `0x00578290` (pure pointer/coord
arithmetic, no RNG):

- **Pass 1** (`0057813d`–`005781a7`): for each cell whose `+0x140 & 0x20` (bridge flag) is
  set: clears bits `& 0xE7` at `cell+0x12C` and `& 0xFFFFFFDC` at `cell+0x140`, sets
  `cell+0x138 = 1` (redraw-dirty), calls `FUN_006da7d0` (ECX = g_Tactical `0x00887324`,
  appends the cell to the Tactical dirty/redraw list), and if `cell+0x120 == 0xFE` calls
  `CellChangeNotify` @ `0x005865f0` (ECX = MapClass `0x87f7e8`).
- **Pass 2** (`005781d9`–`0057827e`): for each cell, gets cell coords via vtable slot `+0x48`,
  shifts leptons→cells (`SAR 0x8` with sign bias), computes the shroud **edge bitmask** via
  `Shroud_EdgeBitmask_Calculator` @ `0x006d8700` (8-neighbour scan of cell flag bit 0x8 /
  bit 0x2 + static LUT `UNK_007f4194`); if the result differs from `cell+0x120`, writes it,
  sets `cell+0x138 = 1`, re-queues redraw (`FUN_006da7d0`), and conditionally
  `CellChangeNotify` when the new value is `0xFE`.

(Note: Pass 2 in decomp indexes `piVar3[0x48]`/`piVar3[0x4e]` — those are the same byte
fields `cell+0x120`/`cell+0x138` the disassembly touches; the decomp's int-index view is
misleading. Trust the disassembly: `0057823f MOV [ESI+0x120],AL` and
`00578245 MOV [ESI+0x138],BL`.)

## Gate / mode condition

**CONFIRMED exactly as stated in the ladder seed.** At `0055b29a`:

```
0055b29a MOV EAX,EDI            ; EDI = [0x00a8ed84] = g_CurrentFrameCounter
0055b29c MOV ECX,0x78           ; 120
0055b2a1 CDQ
0055b2a2 IDIV ECX               ; signed; EDX = frame % 120
0055b2a4 TEST EDX,EDX
0055b2a6 JNZ 0x0055b2c4         ; skip the call if remainder != 0
0055b2a8 MOV ECX,0x87f7e8       ; this = global MapClass
0055b2ad CALL 0x00578100        ; MapClass__RecalcBridgeShroudFlags
```

Gate = `g_CurrentFrameCounter % 0x78 == 0` (every 120 frames), signed `IDIV`.
`g_CurrentFrameCounter` is `[0x00a8ed84]` (the gameplay frame counter loaded into EDI at
`0055b294`; this is the real frame counter, distinct from the perf counter `0x00abcd40`
bumped in the prelude). Otherwise unconditional (no Rules/Scen flag dependency).

## RNG

**Draws NO RNG.** No RNG instance (`Scen->Random`, `g_MainRng`, `g_MapGenRng`) is loaded as
a receiver anywhere in the driver or in any of its callees. Verified callee set:

| Callee | Addr | RNG? |
|---|---|---|
| `MapClass__CellIterator_Next` | `0x00578290` | none (iterator arithmetic) |
| `FUN_006da7d0` (Tactical redraw-queue append) | `0x006da7d0` | none (viewport cull + list append) |
| `CellChangeNotify` | `0x005865f0` | none (radar/object dirty mark, vtable `+0x198` notify) |
| vtable `+0x48` (cell coords getter) | per-cell | none |
| `Shroud_EdgeBitmask_Calculator` | `0x006d8700` | none (neighbour-flag scan + static LUT) |

`rng_stream = none`, `draws_rng = false`. Contributes nothing to the lockstep RNG-draw
order; it only mutates cell flag bytes and the Tactical redraw list, both deterministic.

## Active in YR / TS-legacy

**Active in YR: YES** (conditional on the periodic gate — runs the call every 120 frames in
every skirmish; does meaningful work only when bridge cells exist, which standard YR maps
have). Player-visible output: the shroud edge rendering around bridge cells stays correct.

**NOT Tiberian Sun legacy.** The FogOfWar default-off caveat (TS "previously-seen darkening")
does not apply here — this driver manipulates base shroud-edge bits for bridge cells and is
not gated behind `SpecialFlags`/FogOfWar. Bridges exist in standard YR; the work runs
unconditionally on the 120-frame cadence.

## Caller context

`get_function_callers 0x00578100` returns two callers:
1. `LogicClassPerTickUpdateLiveVector` @ `0x0055afb0` — **this rung** (frame % 120 gate).
2. `FUN_006e1a70` @ `0x006e1a70` — a **TriggerAction handler** (gap-generator / conceal-radius
   at a waypoint); calls RecalcBridgeShroudFlags once after concealing a radius. NOT part of
   the per-tick spine; listed only so the driver's reuse is on record.

## Position in lockstep ladder

Runs after Rung D (ambient lighting fade, `0055b205`–`0055b28c`) and before Rung F (second
lighting channel, `0055b2c4`). Because it draws no RNG, its presence/absence does not shift
any downstream rung's RNG-draw index — but it MUST stay at this position in the per-tick
ordering for cell-flag/redraw-state determinism (it mutates `cell+0x120/0x12C/0x138/0x140`
which later render/vision rungs read).
