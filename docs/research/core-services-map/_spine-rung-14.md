# Spine Rung 14 — N. Laser/draw-segment timer purge (`FUN_005ff390`)

Driver: `0x005FF390` (`FUN_005ff390`)
Body call site: `0x0055b5be` inside `LogicClass::PerTickUpdate` (`LogicClassPerTickUpdateLiveVector` @ `0x0055AFB0`), order #14.

## Verdict summary

| Field | Value |
|---|---|
| Order | 14 (after rung M DiskLaser reverse loop, before rung O `LaserDrawClass__UpdateAllAI`) |
| Gate | **Unconditional reverse loop** (empty when list count = 0). Confirmed. |
| Draws RNG | **No** |
| RNG stream | none |
| Active in YR | **Yes** (conditional on visuals existing) — particle/laser/lightning draw-segments are standard YR |
| TS legacy | No |

## Purpose (one line)

Per-tick lifetime/purge driver for a global `DynamicVectorClass` of short-lived
laser/spark/lightning **draw-segment** objects: ages each entry's frame timer by 8 and
deletes entries whose timer has exceeded the fade window (`> 0x4f`).

## What it walks / does

Verified via `decompile_function 0x005ff390` and `disassemble_function 0x005ff390`.

- Vector globals (a `DynamicVectorClass<T>` triple):
  - `DAT_00ac167c` = element array base (data ptr)
  - `DAT_00ac1688` = element count
  - `DAT_00ac1678` = vector vtable pointer (receiver `ECX = 0xac1678` at the functor call — verified in disasm `005ff3c6: MOV ECX,0xac1678`)
  - (registration path also reads `DAT_00ac1680` capacity, `DAT_00ac1685` allow-grow flag, `DAT_00ac168c` grow-step)
- Reverse walk `for (i = count-1; i >= 0; --i)`:
  1. `entry = DAT_00ac167c[i]`
  2. `entry[+0xc] += 8` (disasm `005ff3a8..005ff3b0`)
  3. if `entry[+0xc] > 0x4f` (signed `CMP EAX,0x50; JL`, i.e. `>= 0x50`) **and** `entry != 0`:
     - call vector find functor `(*(DAT_00ac1678 + 0x10))(&entry)` → returns element index or -1
     - if found, splice it out by shifting the tail down one slot and decrementing `DAT_00ac1688`
     - `FUN_007c8b3d(entry)` — free the object (verified deallocator, see below)

`0x4f` = 79; with +8/tick a draw-segment lives ~10 ticks max once aged (entries are
spawned with a smaller initial timer 0x10..0x3f, see callers below). The threshold/step are
hardcoded constants in the driver, not INI-driven.

## Gate — confirmed unconditional

The spine's "unconditional reverse loop (empty if list empty)" is correct. There is no
mode/flag gate on the driver itself. The only xref to `0x005ff390` is the single
`UNCONDITIONAL_CALL` from `0x0055b5be` (verified via `get_xrefs_to 0x005ff390`). When
`DAT_00ac1688 == 0` the `while (i = i-1, -1 < i)` body never executes.

## RNG — none

Verified via `decompile_function`/`disassemble_function 0x005ff390`: the only calls inside
the loop are:
- the vector find functor `*(DAT_00ac1678 + 0x10)` — this is `DynamicVectorClass::ID/Find`
  (returns element index or -1). The identical functor is used by the remove helper
  `FUN_005ff2d0` (verified via `decompile_function 0x005ff2d0`), which only does a find +
  array-shift. No RNG.
- `FUN_007c8b3d` → `FUN_007c93e8` → `HeapFree(DAT_00b78b9c, 0, ptr)` — the heap
  deallocator / `operator delete` (verified via `decompile_function 0x007c8b3d` and
  `0x007c93e8`). No RNG.

Neither `Scen->Random`, `g_MainRng`, nor `g_MapGenRng` is touched in this rung. **0 draws.**

## What this list holds (callers / spawn path)

Verified via `get_xrefs_to 0x005ff250` (the registration/push-back). Entries are
`operator_new(0x18)` 6-dword objects registered through `FUN_005ff250`:
- `0x0048a620` (`FUN_0048a620`) — spawns a fading visual at a coord; gated by
  `g_ExtraAnimationsEnabled` and a per-source flag (`param_2+0x150`); timer init derived
  from brightness (0x15..0x3f), sets entry `+0x14` frame-flags. Verified via
  `decompile_function 0x0048a620`.
- `0x00435c10` (`FUN_00435c10`) — laser/electric-bolt draw segment: builds the entry with
  `FUN_005ff250(...,0x10)`, computes screen endpoints, blits a line onto `g_PrimarySurface`
  (`vtbl+0x78` GetBuffer / `vtbl+0x38` blit). Forces expiry by writing `+0xc = 0x50/0x59`.
  Verified via `decompile_function 0x00435c10`.
- `0x0062ec5b` in `ParticleSystemClass__AI_Spark` and `0x0062e347` in `FUN_0062e280`
  (a ParticleSystemClass virtual, referenced from vtable `0x007efcb0`) — particle-system
  spark/segment spawns.

Object layout (from `FUN_005ff250`, `decompile_function 0x005ff250`):
`[+0]=coord0 [+4]=coord1 [+8]=coord2 [+0xc]=age-timer(=0 init by ctor, overwritten by
caller) [+0x10]=? [+0x14]=frame-flags`.

Companion passes on the same list:
- `FUN_005fffa0` — "draw all" pass: reverse-walks the list calling `FUN_005ff850` per
  entry (renders each segment). Verified via `decompile_function 0x005fffa0`.
- `FUN_005ff2d0` — single-element remove helper (find + shift). Verified.

## Active-in-YR / TS-legacy

**Active in YR, not TS legacy.** The driver runs unconditionally every live tick. The list
is populated by ParticleSystemClass spark logic and the laser/lightning-bolt draw path —
both standard, player-visible YR visuals (Tesla bolts, Prism/laser beams, spark effects).
The spawn side `FUN_0048a620` is gated by `g_ExtraAnimationsEnabled`, but the laser/bolt
spawner `FUN_00435c10` and the particle-system spawners are not gated by that flag, so the
list is non-empty in normal play and the purge is observably necessary (segments fade out
on a fixed window). No SpecialFlags / FogOfWar gating; no TS-only flag found on the driver
or its callers.

Note: when the list is empty (no active bolts/sparks that tick), the driver is a no-op —
but the rung still occupies its fixed slot in the per-tick order. It must stay in the
ladder at position 14 for lockstep ordering even on ticks where it does nothing.

## Ghidra calls cited

- `decompile_function 0x005ff390`, `disassemble_function 0x005ff390` — driver body
- `get_xrefs_to 0x005ff390` — single unconditional caller `0x0055b5be`
- `decompile_function 0x0055AFB0` — body site, confirms order 14 between rungs M and O
- `decompile_function 0x005ff250`, `get_xrefs_to 0x005ff250` — registration + caller set
- `decompile_function 0x005ff2d0` — remove helper (confirms `+0x10` = find functor)
- `decompile_function 0x005ff850` (via xref) / `decompile_function 0x005fffa0` — draw pass
- `decompile_function 0x007c8b3d`, `decompile_function 0x007c93e8` — free / HeapFree
- `decompile_function 0x0048a620`, `decompile_function 0x00435c10` — spawn callers
- `read_memory 0x00ac1678` — globals zero in static image (runtime-initialized vector)
