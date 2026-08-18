# Spine Rung #2 — "B. Super-weapon recharge timer #1 + redraw"

Part of the `LogicClass::PerTickUpdate` ordered ladder (the per-tick + RNG-draw lockstep
contract). This file documents **rung #2 of 28** only.

Body site: `LogicClassPerTickUpdateLiveVector` @ **0x0055AFB0** (the per-tick fan-out;
sole caller is `Main_Tick` @ 0x0055d360 — verified via `get_function_callers 0x0055AFB0`).

Driver: **0x004f42f0** (`FUN_004f42f0`).

---

## Verdict (one line)

A **redraw/flag-set** rung: when scenario timer slot `Scen+0x11e8` is armed, it decrements
the recharge countdown, disarms the slot, and calls `FUN_004f42f0(this=g_LogicClass, arg=2)`
to mark the **sidebar/tactical view dirty + bump the bridge/redraw counter**. **Draws NO
RNG.** Active in YR. Not TS-legacy.

---

## What the rung is (body site, around order 2)

Disassembly of the rung in `0x0055AFB0` — verified via `disassemble_function 0x0055AFB0`.
EDI = `[0x00a8b230]` = the ScenarioClass-instance pointer (written by scenario-init /
`CCFileClass__Constructor`, verified via `get_xrefs_to 0x00a8b230`; null in the static
image as expected — it is a runtime pointer).

```
0055b17d: MOV EAX,[EDI + 0x11e8]      ; EAX = slot start-frame  (decomp index Scen[0x47a])
0055b183: LEA EDX,[EDI + 0x11e8]      ; EDX -> slot base
0055b189: CMP EAX,EBX  (EBX=0xffffffff)
0055b18b: JZ  0x0055b1d8              ; GATE: if start-frame == 0xffffffff -> SKIP rung
0055b18d: MOV ECX,[EDX + 0x8]         ; ECX = duration  (Scen[0x47c] = byte 0x11f0)
0055b190: MOV EBP,[0x00a8ed84]        ; EBP = g_CurrentFrameCounter
   ... compute remaining = duration - (frame - start); if remaining != 0 -> SKIP ...
0055b1aa: MOV ECX,[EDX]               ; re-read start
0055b1ae: JZ  0x0055b1c6              ; if start==0xffffffff skip the decrement-store
   ... [EDX+8] = max(duration - elapsed, 0)   (clamp the leftover countdown)
0055b1c4: MOV [EDX],EBX               ; disarm slot: start-frame = 0xffffffff
0055b1c6: PUSH 0x2                    ; stack arg = 2
0055b1c8: MOV  ECX,0x87f7e8           ; ECX/this = g_LogicClass singleton
0055b1cd: CALL 0x004f42f0             ; <-- THE DRIVER
```

So the field is the per-scenario **timer slot at byte offset `Scen+0x11e8`** (start-frame
u32) + `Scen+0x11f0` (duration/leftover u32). The "0x47a / 0x47c" in the plan and in the
decompiler output are the **`uint*`-array indices** (`0x47a * 4 = 0x11e8`,
`0x47c * 4 = 0x11f0`) — same field, different unit. Record the **byte offsets 0x11e8 /
0x11f0** as canonical.

**Gate (confirmed, with correction of units):** the rung body runs iff
`*(u32*)(Scen + 0x11e8) != 0xffffffff` (i.e. the slot is armed). The plan's
`g_ScenarioClass_Instance[0x47a] != 0xffffffff` is correct as a *decompiler-index*
expression; in byte terms it is `Scen+0x11e8 != 0xFFFFFFFF`. Unconditional otherwise (no
game-mode gate on this rung).

Note: there is an **earlier, separate** use of the same slot inside the placement
cell-action scan loop (rung A region, `0055b11e`–`0055b159`), which fires
`TechnoClass__ProcessCellAction(0xe,...)` when the countdown has elapsed. That belongs to
rung A's inline loop and consumes the same `Scen+0x11e8/0x11f0` slot read-only; it does not
change rung B's behavior. The rung-B body proper is the `0055b17d`–`0055b1d2` block above.

---

## The driver — `FUN_004f42f0` @ 0x004f42f0

Verified via `decompile_function 0x004f42f0` and `disassemble_function 0x004f42f0`.
Signature is `__thiscall`-shaped: **ECX = `this`**, **one stack arg** (`[ESP+4]`,
`RET 0x4`). The decompiler's `param_1` is ECX/this; its `param_2` is the real stack arg.

At rung B the call is `this = 0x87f7e8` (the LogicClass singleton), `arg = 2`.

```
004f42f0: MOV EAX,[0x00887324]           ; EAX = g_Tactical (DisplayClass)
004f42f5: TEST EAX,EAX
004f42f7: JZ  0x004f4300
004f42f9: MOV byte [EAX + 0xd7d],0x1      ; mark tactical view "needs redraw"
004f4300: MOV EAX,[ESP + 0x4]            ; EAX = arg (= 2 here)
004f4304: TEST EAX,EAX
004f4306: JZ  0x004f431b                  ; arg==0 -> do nothing further
004f4308: CMP dword [ECX + 0xc],0x2       ; is mode field already 2?
004f430c: JZ  0x004f4311                  ;   yes -> don't overwrite (2 is sticky)
004f430e: MOV dword [ECX + 0xc],EAX       ;   else latch arg into [this+0xc] (redraw mode)
004f4311: MOV ECX,0x87f7e8                ; ECX = g_LogicClass
004f4316: CALL 0x00578ac0                 ; MapClass__IncrementBridgeCounter
004f431b: RET 0x4
```

What it walks/does (one line): sets the tactical-redraw dirty bit, latches a sticky
redraw-mode code (`2`) into `[g_LogicClass+0xc]`, and increments a redraw/bridge counter.

Sole callee `0x00578ac0` (`MapClass__IncrementBridgeCounter`, verified via
`decompile_function 0x00578ac0`) is just `++*(char*)(this+0x1158)` — a wrap-around byte
counter on the LogicClass singleton. No RNG, no list walk, no allocation.

The driver is a **general-purpose redraw/flag helper**, not SW-specific — it has ~90 call
sites (`get_xrefs_to 0x004f42f0`), including `ObjectClass__MarkNeedsRedraw`,
`Sidebar_UpdateFromProduction`, `BuildingClass__Unlimbo/Limbo`, `MapClass__BlackoutShroud`,
`CreditsClass__AI`, `PowerClass__AnimationTick`, `StripClass__AI`. Rung B is one caller
that fires it on SW-recharge-timer expiry.

The `[this+0xc]` redraw-mode field is consumed elsewhere (e.g. read at `FUN_006da7d0`,
verified via `get_xrefs_to 0x0087f7f4`).

---

## RNG

**Draws NO RNG.** The driver `0x004f42f0` and its only callee `0x00578ac0` contain no RNG
calls (no `Scen->Random`, no `g_MainRng`, no `g_MapGenRng`) — verified by reading both
function bodies in full (`decompile_function` + `disassemble_function` on 0x004f42f0;
`decompile_function 0x00578ac0`). The only state writes are: tactical dirty bit
`[g_Tactical+0xd7d]`, redraw-mode field `[g_LogicClass+0xc]`, redraw counter
`[g_LogicClass+0x1158]`, and (in the body site) the scenario timer-slot decrement/disarm
at `Scen+0x11e8/0x11f0`. None consume the RNG streams.

This rung therefore **does not advance any RNG stream** — it is RNG-neutral in the lockstep
draw order.

---

## Active in YR? TS legacy?

**Active in YR: yes (conditional)** — fires once each time the `Scen+0x11e8` recharge-timer
slot is armed and its countdown elapses, then disarms the slot. In a normal YR skirmish the
slot is armed by SW/redraw-timer arming paths; on elapse the rung visibly refreshes the
sidebar/tactical view (the recharge readout). The gate `Scen+0x11e8 != 0xffffffff` means it
is dormant (slot disarmed) most ticks and active on the elapse tick.

**TS legacy: no.** The driver and counter are live general-purpose redraw machinery used
all over the active YR codebase (sidebar production updates, building limbo/unlimbo, power
animation). Nothing here is gated behind a TS-only `SpecialFlags` bit.

---

## Plan corrections

- **Field unit:** plan says "Scen+0x47a/0x47c timer". Those are decompiler **`uint*`
  indices**; the **byte offsets are 0x11e8 (start-frame) / 0x11f0 (duration/leftover)**.
  Gate is `*(u32*)(Scen+0x11e8) != 0xFFFFFFFF`. (Verified via `disassemble_function
  0x0055AFB0`: `MOV EAX,[EDI+0x11e8]; CMP EAX,EBX(=0xffffffff); JZ skip`.)
- **Driver signature:** plan's "arg=2" is correct for the **stack arg**, but the driver is
  `__thiscall` — ECX/`this` = `0x87f7e8` (g_LogicClass), set explicitly at the call site
  (`MOV ECX,0x87f7e8`). The `2` is `[ESP+4]`, latched into `[this+0xc]` (sticky).
- **"Super-weapon recharge timer":** accurate as the *trigger* (a recharge/redraw timer
  slot), but the **driver itself is a generic sidebar/tactical redraw + counter helper**,
  not SW-specific logic. No SW state is touched here beyond the redraw flag.
