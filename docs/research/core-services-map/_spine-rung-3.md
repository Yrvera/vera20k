# Spine Rung #3 — C. Clear placement-mode flags (bookkeeping)

**Parent:** `LogicClass::PerTickUpdate @ 0x0055AFB0` (decompiler label
`LogicClassPerTickUpdateLiveVector`). Single caller: `Main_Tick @ 0x0055D360`
(call site `0x0055DC9E`), per `logicclass.md` / `_spine-anchor.md`.

**Position in ladder:** runs immediately after Rung B (SW recharge timer #1 + redraw,
`0055b17d–0055b1d2`) and immediately before Rung D (map lighting fade — ambient,
`0055b205`). It is the unconditional fall-through landing pad for both `JZ 0x0055b1d8`
branches that skip Rung B (`0055b18b`, `0055b1a8`).

Verified this session via `decompile_function 0x0055AFB0` and
`disassemble_function 0x0055AFB0` (image base 0x400000).

---

## Site (exact)

`0055b1d8 – 0055b1fe`, four `MOV byte ptr [base+disp32], 0x0` stores, base reloaded each
time from the ScenarioClass instance pointer global `0x00a8b230` (decompiler name
`g_ScenarioClass_Instance`; `read_memory 0x00a8b230` = `00000000` at static rest — runtime
pointer, expected). Verified via `disassemble_function 0x0055AFB0`:

```
0055b1d8: MOV byte ptr [EDI + 0x34aa],0x0   ; EDI already = [0x00a8b230]
0055b1df: MOV EAX,[0x00a8b230]
0055b1e4: MOV byte ptr [EAX + 0x34a9],0x0
0055b1eb: MOV ECX,dword ptr [0x00a8b230]
0055b1f1: MOV byte ptr [ECX + 0x34ab],0x0
0055b1f8: MOV EDX,dword ptr [0x00a8b230]
0055b1fe: MOV byte ptr [EDX + 0x34be],0x0
```

Decompiler view (`decompile_function 0x0055AFB0`, label `LAB_0055b1d8`):

```c
*(undefined1 *)((int)g_ScenarioClass_Instance + 0x34aa) = 0;
*(undefined1 *)((int)g_ScenarioClass_Instance + 0x34a9) = 0;
*(undefined1 *)((int)g_ScenarioClass_Instance + 0x34ab) = 0;
*(undefined1 *)((int)g_ScenarioClass_Instance + 0x34be) = 0;
```

Note: decompiler's `g_ScenarioClass_Instance` and the assembly's `[0x00a8b230]` are the
same instance-pointer global (the decompiler treats the dereferenced base as a `uint*`,
which is why offsets in C appear as word-indices `[0x47a]` etc. while the byte stores here
use the explicit byte cast).

---

## Purpose (one line)

Per-tick reset of the four ScenarioClass placement-mode / cell-action latch bytes
(`+0x34aa`, `+0x34a9`, `+0x34ab`, `+0x34be`) to 0 so they act as one-shot flags consumed
by Rung A's cell-action scan, never carrying over into the next tick.

**What it walks / does:** nothing — no loop, no list, no array. Four constant-store byte
writes to a single object (the ScenarioClass singleton). Pure bookkeeping; not a driver.
Listed in the ladder for completeness and ordering, not because it does work.

### Latch contract (why these four bytes exist)

The same four bytes are *read* one rung earlier (Rung A, `0055afe6–0055b177`) to decide
which `TechnoClass__ProcessCellAction @ 0x006e53a0` action codes to dispatch:
- `+0x34be` gates action `0x32` (`0055afef` reads `[EDI+0x34be]`).
- `+0x34aa` gates actions `0x1b/0x1c/0x24/0x25` (`0055b01d` reads `[EDI+0x34aa]`).
- `+0x34ab` gates actions `0x2d/0x2e` (`0055b0a3` reads `[EDI+0x34ab]`).
- `+0x34a9` is reset here but not read by Rung A's scan in this function (cleared for
  completeness as part of the same placement-mode flag group).

External events latch these flags ON; Rung A consumes them; Rung C clears them. Confirmed
a concrete writer via `search_byte_patterns "be 34 00 00 01"` → `0x00481ac3`, which
`decompile_function 0x00481ac3` shows is `CrateClass__PickupDispatch` doing
`*(undefined1*)(g_ScenarioClass_Instance + 0x34be) = 1;` on the crate-trigger path (right
before it calls `TechnoClass__ProcessCellAction(0x31, ...)`). So a crate pickup sets the
0x34be latch; the next tick's Rung A scan processes the cell action gated on it; Rung C
zeroes it afterward. `search_byte_patterns "aa 34 00 00 01"` returns setters at
`0x006834f1 / 0x0068351e / 0x00685379 / 0x006896a0 / 0x0068973b / 0x00689940 / 0x006899db`
(scenario-init region) for the `+0x34aa` flag.

---

## Gate / mode condition

**Unconditional.** Confirm of the seed claim. `LAB_0055b1d8` is reached by straight
fall-through from Rung B and is also the explicit target of `JZ 0x0055b1d8` at `0055b18b`
(when `[EDI+0x11e8] == -1`, i.e. SW timer #1 slot empty) and at `0055b1a8` (timer not yet
elapsed). There is no flag, mode, frame-modulo, or count guard around the four stores
themselves — they execute every tick regardless of game mode or game state.

---

## RNG

**Draws no RNG.** No `CALL` of any kind in `0055b1d8–0055b1fe` (verified in
`disassemble_function 0x0055AFB0` — four `MOV` byte stores with interleaved pointer
reloads, then it flows into Rung D's lighting check at `0055b205`). No receiver/ECX is set
up for a draw site. Therefore: not `Scen->Random`, not `g_MainRng`, not `g_MapGenRng` —
**none**. It does not advance the per-tick RNG cursor and is invisible to the lockstep
RNG-draw order (`B→C→E→J→N→P→R→U`).

---

## Active-in-YR / Tiberian Sun legacy

**Active in YR: YES** (the reset executes unconditionally every tick), but it is
**bookkeeping, not player-observable on its own**. It produces no visible effect, no
state-hash contribution beyond keeping the four latch bytes at 0 — its visibility is
entirely indirect, via Rung A correctly treating the placement/cell-action flags as
one-shot. Without this reset, a latched flag would re-trigger a cell action every
subsequent tick (the observable consequence lives in Rung A, not here).

**Tiberian Sun legacy: NO.** The placement-mode / cell-action latch group is live in
standard YR — it backs building-placement ghost commits, super-weapon target-cell
dispatch, and crate-trigger cell actions (`CrateClass__PickupDispatch` writer confirmed).
This is not a gated-off TS-only path.

---

## Lockstep notes

- Order matters relative to Rung A (read) and to any external setter that runs between
  ticks: the flags must be cleared *after* Rung A's scan within the same
  `PerTickUpdate` and *before* the tick returns, which is exactly where Rung C sits.
- Zero RNG impact; reordering Rung C relative to other rungs would not shift the RNG
  cursor, but would change *which* cell actions Rung A sees on subsequent ticks (a
  state-visible, lockstep-relevant effect), so its position is still part of the contract.

---

## Evidence index (Ghidra calls, this session)

- `decompile_function 0x0055AFB0` — full driver; `LAB_0055b1d8` block.
- `disassemble_function 0x0055AFB0` — exact four byte stores `0055b1d8–0055b1fe`, no CALL;
  fall-through + `JZ 0x0055b1d8` predecessors at `0055b18b` / `0055b1a8`.
- `read_memory 0x00a8b230` (4 bytes) = `00000000` — confirms `0x00a8b230` is the runtime
  ScenarioClass instance-pointer global (null at static rest).
- `get_xrefs_to 0x006e53a0` — Rung A's ten `ProcessCellAction` call sites
  (`0055b00a … 0055b159`) that read the flags Rung C clears.
- `search_byte_patterns "be 34 00 00 01"` → `0x00481ac3`;
  `decompile_function 0x00481ac3` = `CrateClass__PickupDispatch`, sets `+0x34be = 1`.
- `search_byte_patterns "aa 34 00 00 01"` → scenario-init `+0x34aa` setters
  (`0x006834f1` etc.).

Confidence: HIGH (content + identity + binding all verified from the function body and
assembly this session). The driver "inline" requested is this exact four-store block; it
matches the prior `_spine-anchor.md` "C. Clear placement-mode flags" note.
