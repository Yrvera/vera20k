# Spine Rung 18 — R. Deferred cell-lighting recalc flush (`FUN_00554d50`)

Driver: `0x00554D50` (`FUN_00554d50`)
Body call site: `0x0055b5f1` inside `LogicClass::PerTickUpdate`
(`LogicClassPerTickUpdateLiveVector` @ `0x0055AFB0`), order #18.

## Verdict summary

| Field | Value |
|---|---|
| Order | 18 — after rung Q (EMP-pulse reverse loop over `DAT_00b04bd4` @ `0x0055b5d9`), before rung S (`FUN_004c54a0` @ `0x0055b5f6`). Confirmed: decomp does NOT swap R/S; the disasm order is Q → R(`FUN_00554d50`) → S(`FUN_004c54a0`). |
| Driver call signature | `__fastcall FUN_00554d50(int param_1=6 /*ms time budget*/, char param_2=0 /*force-flush flag*/)`. **`ECX=6` is param_1, NOT a `this` pointer**; `DL=0` is param_2. |
| Gate | `DAT_00abca50 != 0` (queued cells present) **AND** `DAT_00abca84 == 0` (queue not yet snapshot-complete) for the snapshot phase; plus an internal per-tick time budget (`param_1` ms via `FUN_005b1e40`). Confirmed and refined (see below). The CALL itself at `0x0055b5f1` is unconditional. |
| Draws RNG | **No** |
| RNG stream | none |
| Active in YR | **Yes** (conditional on a dynamic light source being added/removed that tick) |
| TS legacy | No |

## Purpose (one line)

Per-tick, time-budgeted drain of the **deferred cell-relighting queue**: when a dynamic
light source (building/production glow, light-source object) is created or destroyed,
affected cells are queued; this rung recomputes their AltLight values across multiple ticks
without blowing the per-tick budget, then finalizes and triggers a redraw.

## What it walks / does

Verified via `decompile_function 0x00554d50` and `disassemble_function 0x00554d50`.

The queue is a `DynamicVectorClass`-style triple of globals (all runtime-initialized; zero
in the static image — `read_memory 0x00abca44` would show zeros pre-run):
- `DAT_00abca44` = element array base (pointer to array of cell-light-entry pointers)
- `DAT_00abca50` = element count (the gate variable)
- `DAT_00abca48` = capacity, `DAT_00abca4d` = "owns array" / allow-free flag
- `DAT_00abca40` = vector functor table (grow predicate at `+8`, used by the producer)
- progress/phase state: `DAT_00abca7c` (snapshot cursor), `DAT_00abca80` (work counter),
  `DAT_00abca84` (snapshot-complete flag), `DAT_00abca74` (tick counter),
  `DAT_00abca78` (last-redraw tick), `DAT_00829ae8` (redraw cadence), all verified in the
  disasm body.

Each queued entry is a 0x14-byte (20-byte) record built by the producer (see below):
`[+0]=cur AltLight (0x10000) [+4]=tint? [+8/0xa/0xc/0xe]=four ushort color/level words
[+0x10]=packed cell coord`.

Driver flow (three phases in one call):

1. **Snapshot phase** (gated `DAT_00abca50 != 0 && DAT_00abca84 == 0`): reverse-walks the
   queue from cursor `DAT_00abca7c`. For each entry whose `*entry == 0` (not yet
   snapshotted), it resolves the cell via the map cell accessor and captures the cell's
   *current* lighting via `FUN_00484050`, writing the result back into the entry words at
   `+0,+4,+8,+0xa,+0xc,+0xe`. The time-budget escape: every 16th iteration
   (`(cursor & 0xf) == 0xf`) when `param_2 == 0`, it re-reads the timer (`FUN_005b1e40`)
   and breaks the loop if elapsed `>= param_1` ms — i.e. the budget is enforced only when
   not force-flushing. When the work counter `DAT_00abca80` reaches `<= 0`, sets
   `DAT_00abca84 = 1` (snapshot complete).

2. **Finalize/apply phase** (gated `DAT_00abca84 != 0` AND (`param_2 != 0` OR remaining
   budget allows)): forward-walks all `DAT_00abca50` entries, resolves each cell, calls
   `FUN_00483e30` (apply/restore the cell's AltLight from the snapshot words), then frees
   the entry via `FUN_007c8b3d`. Afterwards: `DAT_00abca50 = 0`, frees the array
   (`FUN_007c8b3d(DAT_00abca44)` if owned), clears `DAT_00abca4d/48/84`, and calls
   `FUN_004f42f0(1)` (sidebar/screen redraw flag — same redraw helper rungs B/G use).

3. **Periodic-redraw tail** (gated on elapsed-time check and tick spacing
   `DAT_00829ae8 + DAT_00abca78 < DAT_00abca74`): calls `Math__ftol` (`0x007c5f00`) and
   `FUN_00544ff0`, updates `DAT_00abca78` and recomputes the redraw cadence
   `DAT_00829ae8`. **`FUN_00544ff0` is a stub returning 0** (verified
   `decompile_function 0x00544ff0` → `return 0;`), so `cVar2 == 0` always and the cadence
   is fixed at `(0xffffffcf & 0) + 0x32 = 0x32` (50). No observable branch here.

### Label-drift corrections

- The decomp labels the cell accessor `MapClass__Get_CellClass`, but the **disasm shows
  `MOV ECX,0x87f7e8; CALL 0x005657a0`** (verified `disassemble_function 0x00554d50` at
  `0x00554e22` and `0x00554f68`). `0x87f7e8` is the global MapClass instance (same receiver
  used by rungs X `MapClass__UpdateCrateRegenTimers` and the bridge/lighting rungs). Cite
  the address `0x005657a0`, not the label.
- The relight helpers are `0x00484050` (**snapshot** current cell lighting → out-params)
  and `0x00483e30` (**apply** lighting words into the cell). The spine called them
  "FUN_00484050/FUN_00483e30 relight" — confirmed correct addresses, roles clarified.
- The spine note "decomp swapped R/S order" is **NOT borne out** here: in
  `LogicClassPerTickUpdateLiveVector` the calls appear in true order
  `FUN_00554d50` (`0x0055b5f1`) then `EMPulseClass__UpdateAll`/`FUN_004c54a0`
  (`0x0055b5f6`). No swap.

## Gate — confirmed and refined

The CALL at `0x0055b5f1` is **unconditional** (verified `disassemble_function 0x0055AFB0`:
`XOR DL,DL; MOV ECX,0x6; CALL 0x00554d50` with no preceding branch). The *work* inside is
gated:
- Snapshot phase runs only if `DAT_00abca50 != 0 && DAT_00abca84 == 0`
  (disasm `0x00554d79..0x00554d8e`).
- Apply phase runs only if `DAT_00abca84 != 0` and (`param_2 != 0` or budget remains).
- When `DAT_00abca50 == 0` (no queued cells — the common case) the function still bumps
  `DAT_00abca74` and runs the periodic-redraw tail, then returns. It is effectively a
  no-op for lighting on ticks with no pending relights.

So the spine's "`DAT_00abca50 != 0` (queued cells) + internal time budget
(`FUN_005b1e40`)" is correct for the snapshot phase. `FUN_005b1e40` is the high-resolution
timer source (`timeGetTime`/QPC via `FUN_007c6064`), used to measure elapsed ms against
the `param_1=6` budget — verified `decompile_function 0x005b1e40`. It is **not** a gate by
itself; it bounds how much queue work happens this tick.

## RNG — none

Verified across the full driver call tree. The driver's only calls are:
- `0x005b1e40` — hi-res timer (`timeGetTime`/`FUN_007c6064`), no RNG.
  (`decompile_function 0x005b1e40`)
- `0x005657a0` — MapClass cell accessor (pointer math, no RNG).
- `0x00484050` — snapshot current cell lighting; calls `FUN_00484180` /
  `FUN_00555ac0` / `FUN_00544e70`. (`decompile_function 0x00484050`)
- `0x00483e30` — apply cell lighting; same helper set. (`decompile_function 0x00483e30`)
- `0x00544e70` — `LightConvertClass` palette-remap cache (build/cache via
  `LightConvertClass__Constructor`, `operator_new(0x1b4)`); no RNG.
  (`decompile_function 0x00544e70`)
- `0x007c8b3d` — heap free / `operator delete` (no RNG).
- `0x007c5f00` — `Math__ftol` (float→int, no RNG).
- `0x00544ff0` — stub `return 0` (no RNG). (`decompile_function 0x00544ff0`)

None of `Scen->Random`, `g_MainRng`, nor `g_MapGenRng` is touched. **0 RNG draws.** This
rung is RNG-inert for the lockstep draw order — it consumes zero stream entropy regardless
of how much queue work it does.

## Who fills the queue (producer)

The relight queue is populated by `FUN_00554af0` (verified
`decompile_function 0x00554af0`), itself a `__thiscall(int lightSrc, char mode)`. Callers
(`get_xrefs_to 0x00554af0`):
- `LightSourceClass__ScalarDeletingDestructor` (`0x005551ce`) — a dynamic light source is
  destroyed.
- `CreateProductionAnim` (`0x00554a70`), `FUN_00554a80` (`0x00554a90`), `FUN_00554aa0`
  (`0x00554adc`) — light-source create / production-glow paths.

`FUN_00554af0` is gated by `DAT_00829ae4 != 0` (lighting-enabled) and
`lightSrc->radius (+0x34) <= g_ExtraAnimationsEnabled` (light-radius/extra-anim budget).
It walks the cells inside the light radius; for each affected cell it either applies
directly (`mode==0` → `FUN_00483e30`) or, in deferred mode (`mode!=0`), allocates a
0x14-byte entry (`operator_new(0x14)`) and pushes it to `DAT_00abca44`/`DAT_00abca50`. Note
`FUN_00554af0` itself calls `FUN_00554d50()` (force-flush, no args → `param_1=0,param_2=0`)
at `0x00554b2e` when `mode!=0 && DAT_00abca50 != 0` — i.e. it flushes a previous batch
before queuing a new one. That is the **second caller** of the driver
(`get_function_xrefs 0x00554d50` → `0x0055b5f1` live-tick + `0x00554b2e` producer).

## Active-in-YR / TS-legacy

**Active in YR, not TS legacy.** Dynamic per-cell relighting from light sources is standard
YR rendering: buildings emit ambient light, production animations glow, and destroying a
lit structure must un-light its cells. The producer gate `DAT_00829ae4` (lighting enabled)
is on in normal skirmish, and `g_ExtraAnimationsEnabled` bounds the light radius but does
not disable the system. So the queue is non-empty whenever a lit light source is
created/destroyed that tick, and the player observes the lighting update (cells brighten
near a new structure, darken when one is sold/destroyed). On ticks with no light-source
churn the queue is empty and the rung is a no-op — but it keeps its fixed slot at order 18
for lockstep ordering. No `SpecialFlags` / `FogOfWar` gate; no TS-only flag on the driver
or producer.

## Ghidra calls cited

- `decompile_function 0x00554d50`, `disassemble_function 0x00554d50` — driver body, phases, globals
- `decompile_function 0x0055AFB0`, `disassemble_function 0x0055AFB0` — body site; confirms order Q→R→S and the `MOV ECX,0x6; XOR DL,DL; CALL 0x00554d50` arg setup at `0x0055b5ea`
- `get_function_xrefs 0x00554d50` — two callers: `0x0055b5f1` (live tick), `0x00554b2e` (producer force-flush)
- `decompile_function 0x00554af0`, `get_xrefs_to 0x00554af0` — producer + its callers (LightSource create/destroy, production anim)
- `decompile_function 0x00484050`, `decompile_function 0x00483e30` — snapshot / apply cell lighting helpers
- `decompile_function 0x00544e70` — LightConvertClass palette-remap cache (no RNG)
- `decompile_function 0x005b1e40` — hi-res timer (no RNG)
- `decompile_function 0x00544ff0` — stub `return 0` (tail cadence helper)
