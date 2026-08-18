# Spine Rung 10 — J. BombClass update-all (Ivan bombs / demo charges)

**Rung order:** 10 of 28 in `LogicClass::PerTickUpdate`
**Driver:** `0x00438BF0` `BombClass__UpdateAll` (single-arg `__fastcall`, ECX = `this`)
**Receiver (ECX):** `0x0087f5d8` (global BombClass list/manager)
**Body site:** `0x0055AFB0` `LogicClassPerTickUpdateLiveVector`, call at `0x0055b4e6`

Verified via `decompile_function 0x00438BF0`, `disassemble_function 0x00438BF0`,
`disassemble_function 0x0055AFB0`, `get_function_callers 0x00438BF0`,
`get_xrefs_to 0x0087f5d8`.

---

## Purpose (one line)

Per-tick driver for all active timed attached bombs (Crazy Ivan bombs / Demolition-Truck-
style timed charges): purges detonated/dead entries, advances each bomb's attach sound/
animation, and every 0x2d (45) frames recomputes per-bomb "defuse-detectable" visibility
so the bomb's host object knows whether to draw the attached-bomb indicator.

## Receiver and list structure (this = 0x0087f5d8)

From the driver body (`disassemble_function 0x00438BF0`):
- `this+0x04` — pointer to the bomb-pointer array (vector backing store).
- `this+0x10` — live count.
- `this+0x1c` — pointer to a second array (candidate human-object list) used by the
  proximity scan; count at `this+0x28`.
- `this+0x30` — the 45-frame self-countdown timer for the proximity pass.

## What it walks / does (three passes)

**Pass 1 — compaction / detonation purge** (`00438c02`–`00438c6a`, reverse walk over
`this+0x10`):
- Null entries are removed by left-shifting the tail (in-place vector erase).
- Live entry with `[bomb+0x2c]==0` (no host/attach object): calls virtual slot
  `*(*bomb+0x20)(1)` — the bomb's scalar-deleting-destructor / self-delete — then
  removes the slot. This is the "detonated or orphaned -> destroy" path.

**Pass 2 — attach sound / looping audio** (`00438c6f`–`00438cfe`, reverse walk):
- For each bomb with `[bomb+0x50] != -1`:
  - If host-type byte `[[bomb+0x2c]+0x81] != 0` -> `AnimClass__Detach` (`0x00405d40`),
    clear `[bomb+0x54]`.
  - Else first time (`[bomb+0x54]==0`): `VocClass__PlayAt(bomb+0x3c)` (`0x007509e0`),
    set `[bomb+0x54]=1` — plays the ticking attach sound at the bomb's coord.
  - Else: `AnimClass__UpdateLoopingSound` (`0x00750d40`) with the bomb's loop-sound
    params copied from `[host+0x9c/0xa0/0xa4]`.

**Pass 3 — proximity defuse-detection refresh** (`00438d04`–end, throttled):
- `if [this+0x30] > 0` -> decrement and `return` (runs only 1 frame in 45).
- Otherwise reset `[this+0x30]=0x2d` (45) and reverse-walk the bomb list:
  - Save old visible flag `cVar1 = [[bomb+0x2c]+0x68]`.
  - `HouseClass__IsHumanPlayer` (`0x0050b6f0`) on the bomb's owning house: if a human
    player -> `local_36=1` (always detectable).
  - Else scan the candidate-object array (`this+0x1c`, count `this+0x28`): for each,
    if that object's house `IsHumanPlayer`, compute Euclidean distance between bomb-host
    coords (vt `+0x48`) and the object coords, compare against the object type's range
    `[[obj vt +0x84]+0x5f8] << 8` (range field, leptons = cells*256). If within range,
    set `local_36=1` and break.
  - Write `[[bomb+0x2c]+0x68] = local_36`; if it changed vs `cVar1`, set
    `[[bomb+0x2c]+0x80] = 1` (mark host object for redraw — toggles the visible
    "bomb attached" indicator).

## Exact gate / mode condition

**UNCONDITIONAL.** Confirmed from the body site (`disassemble_function 0x0055AFB0`):
```
0055b4dc: CALL 0x007221b0   ; rung I (Tiberium spread)
0055b4e1: MOV ECX,0x87f5d8  ; receiver
0055b4e6: CALL 0x00438bf0   ; rung J — no preceding test/branch
0055b4f0: CALL 0x0054e4d0   ; rung K
```
No flag/mode/SpecialFlags test guards the call. The driver itself is a no-op when the
list is empty (`this+0x10 == 0` -> all three reverse loops fall through immediately, and
pass 3 still runs the timer decrement/reset but iterates zero bombs). Matches the
ladder's stated gate ("unconditional (empty if no bombs)").

## RNG

**Draws no RNG.** `draws_rng = false`, `rng_stream = none`.

Walked every call in the driver:
- `0x0050b6f0` HouseClass__IsHumanPlayer — reads house flags `+0x1ec`/`+0x1ed` or
  compares ECX to `g_PlayerHouse` (`0x00a83d4c`); no RNG (`disassemble_function 0x0050b6f0`).
- `0x004cac40` Sqrt_Approx and `0x007c5f00` Math__ftol — pure float math
  (`get_function_by_address`).
- `0x00405d40` AnimClass__Detach, `0x007509e0` VocClass__PlayAt, `0x00750d40`
  AnimClass__UpdateLoopingSound — animation/audio housekeeping, no RNG.
- The vtable calls (`+0x20` destroy, `+0x3c` get-linked-object, `+0x48` get-coords,
  `+0x84` get-type) are state reads/lifecycle, not RNG sources.

The proximity check is fully deterministic (integer coord deltas -> Sqrt_Approx ->
ftol -> compare). Contributes nothing to the lockstep RNG-draw order; it occupies its
fixed ordinal slot (10) in the per-tick sequence but does not move the RNG cursor.

## Active-in-YR / Tiberian Sun legacy

**Active in standard YR. Not TS legacy.** `rng draws none; ts_legacy = false`.

`get_xrefs_to 0x0087f5d8` shows the list is populated from `WarheadTypeClass__Detonate`
(`0x0046936b`, inside `0x004690b0`) — the warhead-detonation path that deploys timed
attached bombs (Crazy Ivan's IvanBomb / Demolition Truck) — and re-pointed from
`TechnoClass__Limbo_Helper` / `TechnoClass__Unlimbo` / `Detach_From_All_Lists` when the
host object enters/leaves the world. These are live YR units, so a normal YR skirmish
with a Crazy Ivan (or Demo Truck) produces non-empty bomb lists and the player observes:
the ticking attach sound (pass 2) and the on-host bomb indicator toggling via the
`+0x68/+0x80` visibility/redraw flags (pass 3). When no bombs exist the rung is a no-op
but still holds its ordering slot.

## Confidence

- Driver identity / ECX receiver: HIGH (single caller `0x0055AFB0`, ECX literal
  `0x87f5d8` at `0x0055b4e1`, verified in disassembly).
- Unconditional gate: HIGH (raw disassembly shows no guard).
- No-RNG: HIGH (every callee inspected).
- Bomb-source / active-in-YR: HIGH for the warhead-detonate + limbo/unlimbo xref chain;
  the IvanBomb/Demo-Truck attribution is the standard YR consumer of that path.
