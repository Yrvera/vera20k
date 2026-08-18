# Spine Rung 23 — W. `AlphaShapeClass::PurgeDisabled` (+ one-time gradient LUT init)

Driver: `0x00420E90` (`AlphaShapeClass__PurgeDisabled`)
Body call site: `0x0055b650` (`CALL 0x00420e90`) inside `LogicClass::PerTickUpdate`
(`LogicClassPerTickUpdateLiveVector` @ `0x0055AFB0`), order #23.

## Verdict summary

| Field | Value |
|---|---|
| Order | 23 — after rung V wave-splash (`CALL 0x0053d310` @ `0x0055b64b`), before rung X `MapClass__UpdateCrateRegenTimers` (`MOV ECX,0x87f7e8; CALL 0x0056bbe0` @ `0x0055b655`). Confirmed by disasm: `…0x0053d310 → 0x00420e90 → 0x0056bbe0`. |
| Driver call signature | `void AlphaShapeClass__PurgeDisabled(void)` — no args, no `this`. Plain `CALL 0x00420e90` with no register setup. |
| Gate | **Unconditional.** The CALL at `0x0055b650` has no preceding branch. The one-time gradient-LUT build inside is gated by `DAT_0089a134 == 0` (one-shot init flag); the purge reverse-loop runs every tick (empty when `DAT_0088a100 == 0`). Confirmed — matches spine. |
| Draws RNG | **No** |
| RNG stream | none |
| Active in YR | **Yes** (conditional: non-empty only when objects with `AlphaImage=` exist and one is disabled/limboed that tick) |
| TS legacy | No (AlphaImage lighting overlays are live YR rendering) |

## Purpose (one line)

Per-tick removal of every **AlphaShapeClass** (translucent `AlphaImage=` lighting overlay
attached to a revealed object) whose "disabled" flag is set, plus a one-time build of the
0x10000-entry alpha-blend gradient lookup table on the first call.

## What it walks / does

Verified via `decompile_function 0x00420E90` and `disassemble_function 0x00420E90`.

### Phase 1 — one-time gradient LUT (gated `DAT_0089a134 == 0`)

On the first call, sets `DAT_0089a134 = 1` and fills a 0x10000-byte table at
`0x0088a118` (`&DAT_0088a118`, indexed by a 16-bit value). For each index `i`
(`0..0x10000`), it computes `clamp( ((i >> 8) * (i & 0xff)) / 0x7f , 0, 0xff )` and stores
the byte. The hi byte is the alpha level, the lo byte the source intensity; the product/127
gives the blended output level — i.e. a precomputed `level * alpha / 127` translucency LUT
used by the AlphaShape draw paths. Disasm `0x00420e9a..0x00420f05`.

This identical LUT-build block also appears in `AlphaShapeClass__Constructor`
(`0x00420a71..`, same `DAT_0089a134` one-shot guard), so whichever runs first (first
AlphaShape constructed, or first PurgeDisabled tick) builds the table; the other is a no-op.
Verified `decompile_function 0x00420a00` (entry `0x00420960`).

### Phase 2 — purge reverse-loop (every tick)

Reverse-walks the global AlphaShape array `DAT_0088a0f4` (base ptr) of count
`DAT_0088a100`, from `count-1` down to `0`. Disasm `0x00420f07..0x00420f36`:

```
iVar2 = DAT_0088a0f4; iVar4 = DAT_0088a100;
while (--iVar4 >= 0) {
    piVar1 = *(int**)(iVar2 + iVar4*4);        // entry ptr
    if (piVar1[0xf] (byte @ +0x3c) != 0 && piVar1 != 0)
        (**(code**)(*piVar1 + 0x20))(1);        // virtual call, vtable slot 0x20, arg=1
    iVar2 = DAT_0088a0f4;                        // re-read base (array may shrink)
}
```

- Per-entry "disabled" predicate: byte at object `+0x3c` (`piVar1[0xf]`) `!= 0`.
  Set to 0 at construction (`*(undefined1 *)(param_1 + 0xf) = 0` in the constructor).
- The removed entry is dispatched through **vtable slot 0x20** with arg `0x1`. The
  AlphaShapeClass primary vtable is at `0x007e32a4` (`list_globals
  vtable__AlphaShapeClass`); slot 0x20 (byte offset 0x20 = entry index 8) reads
  `0x00421730` (`read_memory 0x007e32a4` len 64 → DWORD @ +0x20 = `30 17 42 00`).
- `0x00421730` is `AlphaShapeClass__Destructor` (`get_function_by_address 0x00421730`,
  `decompile_function 0x00421730`). Arg `0x1` is the scalar-deleting-destructor free bit:
  the destructor's tail does `if (local_4 & 1) FUN_007c8b3d(param_1)` → frees the object.

So the destructor (a) restores the four AlphaShape vtable pointers, (b) finds the entry's
index in `DAT_0088a0f4` via the DynamicVector functor (`DAT_0088a0f0 + 0x10`) and
compacts the array down (`DAT_0088a100--`), (c) does the same for a second registration
vector `DAT_00b0f724` / `DAT_00b0f730`, (d) calls
`AbstractClass__Destructor_ResetVtables` (`0x004101f0`), (e) frees the object via
`FUN_007c8b3d`. That is why PurgeDisabled re-reads `DAT_0088a0f4` after each removal — the
array base/count can change. Verified `decompile_function 0x00421730`.

### The label-hint `0x0055b650`

The spine described the removal target as "vt+0x20 @ `0x0055b650`". **`0x0055b650` is the
body CALL-site address** (`CALL 0x00420e90` inside `LogicClassPerTickUpdateLiveVector`),
**not** the vt+0x20 target. The actual vt+0x20 resolves to `AlphaShapeClass__Destructor`
`0x00421730` (verified from the vtable bytes, above). Recorded as label drift.

## Gate — confirmed

The CALL is **unconditional** — `disassemble_function 0x0055AFB0` shows
`0055b64b: CALL 0x0053d310` (rung V) immediately followed by `0055b650: CALL 0x00420e90`
with no branch between them, then `0055b655: MOV ECX,0x87f7e8` / `0055b65a: CALL 0x0056bbe0`
(rung X). Inside the driver:
- LUT build runs only once (`DAT_0089a134` one-shot).
- The purge loop runs every tick; when `DAT_0088a100 == 0` (no AlphaShapes) the loop body
  never executes and the function returns immediately — a no-op on ticks with no overlays.

## RNG — none

**0 RNG draws.** Verified across the driver and its only reachable callees:
- The driver has no static callees (`get_function_callees 0x00420E90` → none); its only
  call is the indirect vt+0x20 → `AlphaShapeClass__Destructor`.
- `AlphaShapeClass__Destructor` (`0x00421730`) calls only
  `AbstractClass__Destructor_ResetVtables` (`0x004101f0`, vtable resets) and
  `FUN_007c8b3d` (heap free), plus two DynamicVector "find index / compact" indirect calls
  (`DAT_0088a0f0+0x10`, `DAT_00b0f720+0x10`) — pointer bookkeeping, no RNG.
  (`get_function_callees 0x00421730`)
- `FUN_007c8b3d` (`0x007c8b3d`) → `FUN_007c93e8` (`0x007c93e8`) is the C runtime `free()`
  (`HeapFree(DAT_00b78b9c,…)` bracketed by `_lock/_unlock` `FUN_007cd9f5(9)/FUN_007cda56(9)`).
  No RNG. (`decompile_function 0x007c8b3d`, `decompile_function 0x007c93e8`)

None of `Scen->Random`, `g_MainRng`, nor `g_MapGenRng` is referenced. This rung is
RNG-inert for the lockstep draw order regardless of how many overlays it purges.

## What an AlphaShapeClass is / who creates them

AlphaShapeClass is the runtime instance of an object's `AlphaImage=` (art.ini) translucent
lighting sprite — a soft additive/translucent glow overlay drawn on the tactical view
(e.g. light-post glow, certain structures' ambient light). Evidence:
- Constructor caller is `ObjectClass__Reveal` (`0x005f4ec0`,
  `get_function_callers 0x00420a00`). In `ObjectClass__Reveal` an AlphaShape is allocated
  (`operator_new(0x40)` then `AlphaShapeClass__Constructor(obj, x, y)`) **only when**
  `piVar5[0x2b] != 0` — i.e. the object's TypeClass has a nonzero AlphaImage handle. The
  block also calls `TacticalClass__DirtyScreenRect`, confirming it is a screen overlay.
  (`decompile_function 0x005f4ec0`)
- Sibling functions on the same global array confirm the family: `AlphaShapeClass__
  DrawAll_WithMask` (`0x00420f40`, caller `Tactical_layer_shroud_edges 0x006d3660`),
  `AlphaShapeClass__DrawAll_NoMask`, `AlphaShapeClass__InitGlobalArray`,
  `AlphaShapeClass__ClearGlobalArray`, `AlphaShapeClass__Load/Save` — all read/write
  `DAT_0088a0f4`. (`get_xrefs_to 0x0088a0f4`)
- The object stores the AlphaShape vtable ptrs at `+0`, the parent object at `+0x24`
  (`param_1[9]`), screen x/y at `+0x28/+0x2c`, the disabled flag at `+0x3c`
  (`param_1[0xf]`). (`decompile_function 0x00420a00`)

The disabled flag at `+0x3c` is set elsewhere when the parent object is concealed / goes
to limbo / is removed; PurgeDisabled is the deferred reaper that deletes those overlays at
the fixed order-23 slot rather than mid-frame during conceal.

## Active-in-YR / TS-legacy

**Active in YR, not TS legacy.** `AlphaImage=` is a stock art.ini key used by standard YR
objects (light posts and similar ambient-light props). When such an object is revealed an
AlphaShape overlay is created (`ObjectClass__Reveal`), drawn each frame by the DrawAll
paths, and purged here when disabled. No `SpecialFlags` / `FogOfWar` gate on the driver or
its constructor path — the only gate is data presence (`AlphaImage` set). On ticks with no
overlay disabled the loop is empty; the rung keeps its fixed slot at order 23 for lockstep
ordering. Player-visible effect: the soft light overlays appear/disappear with their parent
object's reveal/conceal lifecycle.

## Ghidra calls cited

- `decompile_function 0x00420E90`, `disassemble_function 0x00420E90` — driver: one-time LUT + reverse purge loop, `[+0x3c]` predicate, vt+0x20 dispatch with arg 1
- `disassemble_function 0x0055AFB0` — body site: `0x0055b64b` (rung V) → `0x0055b650 CALL 0x00420e90` (rung W, unconditional) → `0x0055b655/5a` (rung X); confirms no gate around the call
- `decompile_function 0x0055AFB0` — body site decomp (`AlphaShapeClass__PurgeDisabled();` between `FUN_0053d310()` and `MapClass__UpdateCrateRegenTimers()`)
- `get_function_callers 0x00420E90` — sole caller is the live-tick body
- `list_globals vtable__AlphaShapeClass` → `0x007e32a4`; `read_memory 0x007e32a4` (len 64) → slot 0x20 = `0x00421730`
- `get_function_by_address 0x00421730`, `decompile_function 0x00421730` — vt+0x20 target = `AlphaShapeClass__Destructor` (scalar-deleting, arg 1 frees)
- `get_function_callees 0x00420E90` (none), `get_function_callees 0x00421730` (`0x004101f0`, `0x007c8b3d`)
- `decompile_function 0x007c8b3d`, `decompile_function 0x007c93e8` — heap free / `HeapFree` (no RNG)
- `decompile_function 0x00420a00` (`AlphaShapeClass__Constructor`, entry `0x00420960`) — vtables, `+0x3c` disabled flag init, same LUT-build guarded by `DAT_0089a134`
- `get_function_callers 0x00420a00` → `ObjectClass__Reveal 0x005f4ec0`; `decompile_function 0x005f4ec0` — creation gated by parent TypeClass AlphaImage `!= 0`, calls `TacticalClass__DirtyScreenRect`
- `get_xrefs_to 0x0088a0f4` — AlphaShapeClass family (Constructor/Destructor/DrawAll/InitGlobalArray/ClearGlobalArray) all share the global array
- `get_xrefs_to 0x0089a134` — one-shot LUT flag written by Constructor + PurgeDisabled only
