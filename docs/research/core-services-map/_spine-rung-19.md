# Spine Rung 19 — S. Timed-effect expiry purge (EMPulseClass expiry sweep)

Part of `LogicClass::PerTickUpdate` (the per-tick spine) at `LogicClassPerTickUpdate @ 0x0055AFB0`.
This rung's driver is `0x004C54A0` (Ghidra label `EMPulseClass__UpdateAll`).

## Verdict (one line)

EMP-pulse field expiry sweep: reverse-walks the live EMPulseClass list and **destroys** (scalar-deleting-destructor) every pulse whose `creation_frame + duration <= current_frame`, which also un-applies its cell flags and EMP locks. **No RNG.** **Tiberian Sun legacy — inert in stock YR** (the EMP warhead is `;gs disabled in code`; the list is always empty, so the loop never iterates).

## Driver `0x004C54A0` — body (label `EMPulseClass__UpdateAll` is the correct class, but role is *expiry destroy*, not generic "update")

Verified via `decompile_function 0x004C54A0` and `disassemble_function 0x004C54A0`:

```c
void EMPulseClass__UpdateAll(void) {
  int iVar2 = DAT_008a3880;                              // live EMPulse count
  while (iVar2 = iVar2 + -1, -1 < iVar2) {               // reverse walk
    piVar1 = *(int **)(DAT_008a3874 + iVar2 * 4);        // entry ptr
    if ((piVar1[0xc] + piVar1[0xb] <= g_CurrentFrameCounter) && (piVar1 != 0)) {
      (**(code **)(*piVar1 + 0x20))(1);                  // vt+0x20 with arg 1
    }
  }
}
```

Disassembly confirms the CALL is hidden inside the loop (decomp shows it as an indirect call):
- `004c54b6 MOV EDX,[ECX+0x30]` (duration), `004c54b9 MOV EAX,[ECX+0x2c]` (creation frame), `004c54bc ADD EDX,EAX`
- `004c54be MOV EAX,[0x00a8ed84]` (g_CurrentFrameCounter), `004c54c3 CMP EAX,EDX`, `004c54c5 JL` (skip if now < expiry)
- `004c54cb MOV EAX,[ECX]` (vtable), `004c54cd PUSH 0x1`, `004c54cf CALL [EAX+0x20]` — **the destroy CALL appears only in disassembly.**

Field/index mapping (byte offset = int-index*4):
- `[0xb]` = byte `+0x2c` = creation frame (set to `g_CurrentFrameCounter` in the constructor) — verified `disassemble_function 0x004c52b0` (`004c52ca MOV EDX,[0x00a8ed84]; 004c52d7 MOV [ESI+0x2c],EDX`).
- `[0xc]` = byte `+0x30` = duration (constructor param 3) — verified same disassembly (`004c52c3/004c52d0 MOV [ESI+0x30],EAX`).
- Gate algebra: `duration + creation_frame <= now` → effect lifetime elapsed → expire.

### Globals (verified inline)
- `DAT_008a3880` = live EMPulseClass count; `DAT_008a3874` = pointer array base. Both written by the EMPulseClass constructor (`get_xrefs_to 0x008a3880`: writes at `004c5340` ctor, `004c5b1f` dtor).
- `g_CurrentFrameCounter` = `0x00a8ed84` (verified: it is the same global read at the gate and stored as the creation frame).

## What `vt+0x20` actually is — the **scalar deleting destructor** (it DESTROYS the pulse)

Primary EMPulseClass vtable = `0x007e87a8` (read from ctor: `004c52da MOV [ESI],0x7e87a8`). Slot `+0x20` (8th entry) = `0x004c5ac0` — verified via `read_memory 0x007e87a8` (slot bytes `c0 5a 4c 00`).

`decompile_function 0x004c5ac0` / `disassemble_function 0x004c5ac0` → `EMPulseClass__ScalarDeletingDestructor`. Called with arg `1` (the `PUSH 0x1`), which sets the "free the object" flag. It:
1. Restores the 4 vtables.
2. If `g_GameActive` (byte `0x00a8e9a0`) != 0 → calls `FUN_004c58c0` (the **un-apply**, see below).
3. `0x007258d0` `Detach_From_All_Lists` (DL=1) — observer/removal-listener notification only.
4. Compacts itself out of `DAT_008a3874[]` and decrements `DAT_008a3880`.
5. `FUN_007c8b3d` = `operator delete` (free), because the arg-1 deletion flag is set.

### Un-apply `FUN_004c58c0` — clears EMP cell flags (no RNG)
`decompile_function 0x004c58c0`: walks the square radius `[-r..+r]×[-r..+r]` around `(+0x24,+0x26)`, keeps cells with `dx²+dy² <= r²` (`r` = field `+0x28`), and for each valid cell clears bit `0x80000` in cell flags `+0x140` (`& 0xfff7ffff`). This is the EMP "disabled" cell mark cleanup. No RNG, no unit re-enable here (EMP lock on the techno expires on its own timer field).

## RNG — driver draws NONE

- The driver path (expire → scalar-deleting-destructor → `FUN_004c58c0` un-apply → `Detach_From_All_Lists` → free) contains **no RNG draws**. `Detach_From_All_Lists` (`decompile_function 0x007258d0`) is pure observer-notification/unregister; `FUN_004c58c0` only clears cell flags.
- The **one** RNG draw in the whole EMPulseClass family is in **`EMPulseClass__Apply` (`0x004c54e0`)**, which runs at **construction time** (rung A → ProcessCellAction → EMP weapon launch), NOT in this rung: `Random__RandomRanged(0,0x19)` once per affected vehicle, to pick the EMP-spark anim frame before `AnimClass__Constructor(Rules+0x17f4, ...)`. Verified `decompile_function 0x004c54e0`. This draw belongs to whatever tick the pulse is created on — it is not part of rung 19.
- `rng_stream` for that construction-time draw: `Random__RandomRanged` with no explicit ECX receiver shown at the callsite → the global `Scen->Random` path (the standard gameplay RNG), but this is **out of scope for rung 19** since the driver itself never draws.

## Gate / mode condition (confirmed)

**Unconditional reverse loop**, exactly as the ladder states. The spine calls `0x004c54a0` every tick with no surrounding gate:
- Spine site verified in `disassemble_function 0x0055AFB0`: `0055b5f1 CALL 0x00554d50` (rung R/18) → **`0055b5f6 CALL 0x004c54a0` (rung S/19)** → `0055b5fb MOV EDI,[ESP+0x10]` then the MAIN object-vector tick (rung T/20). Order in the ladder is correct.
- The only "gate" is internal: `DAT_008a3880 > 0` makes the loop iterate; per-entry expiry test `[+0x30]+[+0x2c] <= now`. With an empty list the loop body never runs.

## Active-in-YR — NO (inert). Tiberian Sun legacy.

- `ini/rulesmd.ini:26413` `[EMPuls];gs disabled in code` — the EMP warhead is explicitly **disabled in code** by Westwood; all tuning keys commented (`;Spread=11`, etc.), only `EMEffect=yes` left.
- `[EMPulseWeapon]` exists (`rulesmd.ini:23965`) but is **never assigned** to any unit `Primary=/Secondary=/Weapon=`. Grep of `=EMPulseWeapon`/`=EMPuls`: only `EMPulseWarhead=EMPuls` (line 587, "warhead used by falling nuke missile" — disabled nuke-pulse legacy), a Warheads-list entry (line 2877), and `[EMPulseWeapon] Warhead=EMPuls` (line 23969). No live emitter.
- Consequence: in a standard YR skirmish nothing ever constructs an EMPulseClass, so `DAT_008a3880` stays 0 and rung 19's loop never iterates → **no player-visible effect**. The rung stays in the ladder (called unconditionally each tick) but is dormant.
- Origin: EMPulseClass is the Tiberian Sun EMP-cannon / nuke-EMP-blast field carried into gamemd.exe. Classic TS-legacy-disabled-in-YR pattern.

## Lockstep contract note

Because the driver never iterates in stock YR (empty list) and never draws RNG even when it does iterate, rung 19 consumes **zero RNG draws** from the per-tick stream. Its slot in the order still matters only as a stable boundary between rung 18 (`0x00554d50` shroud/lighting flush) and rung 20 (MAIN object-vector tick) — no RNG-draw-order impact.

## Evidence index

- `decompile_function 0x004C54A0`, `disassemble_function 0x004C54A0` — driver body + hidden destroy CALL.
- `disassemble_function 0x0055AFB0` — spine call site `0055b5f6 CALL 0x004c54a0`, order between rung R and rung T.
- `disassemble_function 0x004c52b0` — constructor: vtable `0x7e87a8`, fields `+0x2c` creation frame / `+0x30` duration, registration into `DAT_008a3874`.
- `read_memory 0x007e87a8` — vtable slot `+0x20` = `0x004c5ac0`.
- `decompile_function 0x004c5ac0` / `disassemble_function 0x004c5ac0` — scalar deleting destructor (the "detonate"/destroy).
- `decompile_function 0x004c58c0` — un-apply (clears cell flag bit 0x80000), no RNG.
- `decompile_function 0x004c54e0` — EMPulseClass__Apply (construction-time): the only RNG draw `Random__RandomRanged(0,0x19)`, NOT in this rung.
- `decompile_function 0x007258d0` — Detach_From_All_Lists, no RNG.
- `get_xrefs_to 0x008a3874`, `get_xrefs_to 0x008a3880`, `get_function_callers 0x004C54A0` (only caller = spine).
- `ini/rulesmd.ini` lines 26413 (`[EMPuls];gs disabled in code`), 23965 `[EMPulseWeapon]`, 587 `EMPulseWarhead=EMPuls`; grep `=EMPuls` shows no live emitter.
