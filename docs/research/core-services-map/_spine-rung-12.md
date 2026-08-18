# Spine Rung 12 — L. TeamClass cull-and-tick (build temp list, then tick)

**Status:** VERIFIED from binary (Ghidra live, image base 0x400000).
**Body site:** `LogicClassPerTickUpdateLiveVector` @ `0x0055AFB0` — build loop `0x0055b4f5–0x0055b582`, tick loop `0x0055b582–0x0055b5a1`.
**Driver:** `TeamClass::AI` @ `0x006e9140` (vtable slot `+0x5c` on each TeamClass).

---

## Purpose (one line)

Tick the AI team/script state machine: cull the team registry into a temp list, then per surviving team run its mission-script opcode handler (Patrol/Attack/Move/random-move/SW-launch/etc.) which drives the team's member units.

## What it walks / does

Two sequential phases, verified in the body disassembly of `0x0055AFB0`:

1. **Build temp list** (`0x0055b4f5–0x0055b582`):
   - `0x0055b4fd CALL 0x0055bb40` constructs a stack-local DynamicVector (the temp). `FUN_0055bb40` is a plain DynamicVector ctor (sets vtable `PTR_FUN_007e9f84`, allocs `count<<2` only when seeded with a capacity; here seeded `(0,0)` → empty, lazy-grow) — it does **not** walk anything itself. (verified via `decompile_function 0x0055bb40`).
   - Loop walks the global TeamClass registry: array base `g_TeamClass_Array` `0x008b40ec`, count `g_TeamClass_Array_Count` `0x008b40f8` (`0x0055b502 MOV EAX,[0x008b40f8]`, `0x0055b577 MOV ECX,[0x008b40f8]`). For each entry it copies the pointer into the temp; capacity is fixed at `local_4 = 10` (`MOV ECX,0xa` @ `0x0055b507`).
   - The "predicate `PTR_FUN_007e9f64` vt+0x8" (`= FUN_004ea1c0` @ `0x004ea1c0`) is **only** called when the temp must grow past current capacity (`0x0055b558 CALL [EAX+0x8]`). It is the DynamicVector grow/resize callback (alloc + copy-forward), **not** a per-team gameplay filter, and draws no RNG. (verified via `decompile_function 0x004ea1c0`). The fast path (`local_8 < iVar6`) copies directly with no callback.

2. **Tick temp list** (`0x0055b582–0x0055b5a1`):
   - Forward loop over the temp count; for each `obj`: `0x0055b593 MOV EDX,[ECX]` (vtable) `0x0055b595 CALL [EDX+0x5c]` → `TeamClass::AI`.
   - The cull-into-temp-then-tick pattern (rather than ticking the live array) is the classic "snapshot before mutation": a team can disband/spawn sub-teams during its own AI tick (e.g. script case 0x12 spawns a new team via `TeamClass__Constructor`, case 0x11/0x1f/0x3c-0x40 disband members), so the live registry is unsafe to iterate directly.

## Gate / mode condition

**Unconditional except count.** No game-mode gate. Build loop runs only if `g_TeamClass_Array_Count > 0` (`0x0055b50c CMP EAX,EDI` / `JLE 0x0055b582`); tick loop runs only if temp count > 0 (`0x0055b588 TEST EAX,EAX` / `JLE 0x0055b5a1`). So the prompt's stated gate "g_TeamClass_Array count > 0" is **confirmed**. There is NO `g_GameMode` check on this rung (contrast rung U / AnimClass, which IS `g_GameMode != 0 && != 5`). (verified via `disassemble_function 0x0055AFB0`).

The per-team gating happens *inside* `TeamClass::AI`: it reads team flags at `this+0x77/0x79/0x7a/0x7b/0x7d/0x7f/0x81/0x82/0x83` (active/suspended/disbanded/recruiting bits) and the team's script CD-timer (`this+0x19`/`this+0x1b`) before advancing — a team whose timer hasn't elapsed early-returns the remaining time without running its opcode.

## RNG

**Draws RNG: YES (conditionally), stream = Scen->Random.**

- `TeamClass::AI` itself contains no direct RNG draw, but its switch dispatches to script helpers. The verified RNG path is **script opcode 0x36** → `TeamClass__Convoy_Script_Random_Move` @ `0x006efa10`.
- Draw site `0x006efacb–0x006efadc`:
  ```
  006efacb: MOV EAX,[0x00a8b230]      ; g_ScenarioClass_Instance
  006efad0: PUSH 0xff                 ; max = 255
  006efad5: PUSH ESI                  ; min = 0  (ESI==0 on this branch)
  006efad6: LEA ECX,[EAX + 0x218]     ; receiver = Scen+0x218  → Scen->Random
  006efadc: CALL 0x0065c7e0           ; Random__RandomRanged
  ```
  The receiver ECX is `g_ScenarioClass_Instance + 0x218` (the ScenarioClass embedded lagged-Fibonacci RNG), i.e. **Scen->Random**, the synchronized lockstep stream — NOT `g_MainRng` and NOT `g_MapGenRng`. (verified via `disassemble_function 0x006efa10`; `decompile_function 0x0065c7e0` confirms `0x0065c7e0` is `RandomRanged` over a lagged-Fibonacci state object passed as `param_1`/ECX).
- **Count / for what:** exactly **one** draw, `RandomRanged(0,0xff)`, only on the branch where the team has no enemy-house reference (`iVar5 == 0`); the byte is `<<8` into a 16-bit facing/angle used to pick a random move destination. If an enemy house IS referenced, that branch instead uses `Math__atan2` toward it (no draw). So: **0 or 1 Scen->Random draw per ticked team, per tick, only for teams executing the random-move script opcode.**
- Other switch cases reviewed for obvious draws: `FUN_00747370` (cases 0x2c/0x2d, "pick TRUCKA/TRUCKB unit type") is a deterministic string-name lookup over `g_UnitTypeClass_Array` — no RNG (verified via `decompile_function 0x00747370`). Convoy attack/patrol/follow/move-to-cell helpers select by scored search, not RNG, on the cases inspected. (Note: not every one of the ~0x40 opcode helpers was exhaustively decompiled; the random-move path is the confirmed RNG consumer. Other opcode helpers UNCHECKED for RNG but none observed in the dispatch body itself.)

## Active-in-YR / Tiberian Sun legacy

- **Active in YR: YES.** TeamClass is the AI team/scripting engine. In a normal YR skirmish, AI (computer) houses create teams from TeamTypeClass; map triggers/`[Teams]` also create them. The rung fires every tick that any team exists. Player-visible effect: AI attack waves, scouting, harvester/engineer micro, super-weapon launches — all flow through these script opcodes.
- **NOT Tiberian Sun legacy.** The script-opcode set is current YR: e.g. cases `0x37/0x38/0x39` = `AI__SuperLaunchCheck_*` (single/dual SW), `0x20/0x21` LightningStorm start/op, `0x2e–0x30/0x3a/0x3b/0x35/0x36` convoy attack/move variants. These are live YR behaviors, not dead TS branches.
- **Project scope caveat (not a parity statement):** per project memory `feedback_no_ai_yet`, AI-system implementation is deferred at the current stage. That is a *scheduling* decision; gamemd unambiguously runs this rung in standard YR skirmish, so it remains a real rung in the lockstep order and a future parity obligation. It must NOT be dropped from the per-tick ORDER.

## Lockstep-order notes (context for neighbors)

- Within the spine body, this rung sits immediately after `FUN_0054e4d0` (rung K, anim re-anchor) at `0x0055b4f0` and immediately before the DiskLaser reverse-walk (rung M) at `0x0055b5a1`.
- RNG-order consequence: any Scen->Random draw from a team's random-move opcode is consumed *here*, after Tiberium growth/spread (rungs H/I, which also touch Scen->Random per project RNG-routing notes) and before combat/object ticks (rung T). The draw count is data-dependent (number of teams running opcode 0x36 this tick), which is itself deterministic given identical lockstep state — so it is lockstep-safe as long as team iteration order (registry insertion order) and the cull are reproduced exactly.

## Ghidra calls cited

- `decompile_function 0x0055AFB0` — body (build + tick loops, gate).
- `disassemble_function 0x0055AFB0` — exact loop bounds, count globals, vt+0x5c dispatch.
- `decompile_function 0x0055bb40` — temp DynamicVector ctor (no walk).
- `decompile_function 0x004ea1c0` — vt+0x8 = DynamicVector grow callback (not a filter, no RNG).
- `get_xrefs_to 0x008b40ec` — confirms `0x008b40ec` is the TeamClass array (TeamClass__Constructor xref).
- `get_function_by_address 0x006e8b94` + `disassemble_function 0x006e8a90` — TeamClass ctor; vtable write `*this = 0x007f4730` @ `0x006e8b32`.
- `read_memory 0x007f478c` — vtable slot `+0x5c` = `0x006e9140`.
- `decompile_function 0x006e9140` — TeamClass::AI script switch.
- `list_functions_enhanced name_contains=Random_Move` — `TeamClass__Convoy_Script_Random_Move = 0x006efa10`.
- `decompile_function 0x006efa10` + `disassemble_function 0x006efa10` — RNG draw site, receiver `Scen+0x218`.
- `decompile_function 0x0065c7e0` — `Random__RandomRanged` on lagged-Fibonacci RNG state.
- `decompile_function 0x00747370` — deterministic unit-type name lookup (no RNG).
