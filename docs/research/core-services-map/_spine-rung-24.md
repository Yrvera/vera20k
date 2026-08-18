# LogicClass::PerTickUpdate — Rung 24 (X): MapClass::UpdateCrateRegenTimers

**Status:** VERIFIED from binary this session.
**Parent:** `LogicClass::PerTickUpdate` @ `0x0055AFB0` (label `LogicClassPerTickUpdateLiveVector`).
**Authority:** binary -> Ghidra. Body site keyed to **disassembly** at
`disassemble_function 0x0055AFB0`; driver keyed to `disassemble_function 0x0056BBE0`.

---

## Order / position

- **Order:** 24 of 28. Runs immediately **after** Rung W (`AlphaShapeClass__PurgeDisabled`
  @ `0x00420E90`, body call `0055b650`) and immediately **before** Rung Y (Tactical/
  DisplayClass `g_Tactical->vt+0x5c`, body `0055b65f`–`0055b667`).

## Body site (exact)

`disassemble_function 0x0055AFB0`, instructions `0055b650`–`0055b667`:

```
0055b650  CALL 0x00420e90            ; <-- Rung W: AlphaShapeClass__PurgeDisabled
0055b655  MOV  ECX,0x87f7e8          ; receiver = g_MapClass_Instance (0x0087f7e8), __fastcall this
0055b65a  CALL 0x0056bbe0            ; <-- Rung X: MapClass__UpdateCrateRegenTimers   (THIS RUNG)
0055b65f  MOV  ECX,[0x00887324]      ; <-- Rung Y: g_Tactical
0055b665  MOV  EAX,[ECX]
0055b667  CALL [EAX + 0x5c]          ; Rung Y dispatch
```

- **Spine-prompt address `0055b65a` note:** the prompt described the placement RNG as being
  at `0055b65a` — that address is actually the **CALL site of the driver** (the `CALL
  0x0056bbe0` instruction inside the parent), not the RNG draw. The RNG draws live two levels
  down inside `MapClass__PlaceCrateAtRandomCell` (`0x0056bd40`). Corrected below.
- **Receiver confirmed:** `ECX = 0x0087f7e8` (set at `0055b655`), the `MapClass` singleton
  instance, passed as the `__fastcall` `this` pointer. Matches spine "ECX=0x87f7e8".

## Purpose (one line)

Per-tick service of the 256-slot **crate respawn-timer table** on the MapClass: any slot whose
countdown has elapsed is cleared (preserving any leftover timer remainder) and a **new crate is
placed at a random map cell**, keeping the active-crate count topped up while crates are enabled.

## Driver — `MapClass__UpdateCrateRegenTimers` @ `0x0056BBE0`

`disassemble_function 0x0056BBE0` / `decompile_function 0x0056BBE0`:

- **Walks** `this+0x158`, **0x100 (256) slots**, **stride 0x10 bytes** (`LEA ESI,[EDI+0x158];
  MOV EBX,0x100; ... ADD ESI,0x10; DEC EBX; JNZ`). Forward walk.
- Per slot, layout used (slot is 16 bytes):
  - `slot+0x0`  = `last-set frame` (`-1`/0xffffffff = "timer paused/not running").
  - `slot+0x8`  = `timer remaining` (frames).
  - `slot+0xc`  = a 2-short cell coordinate / cell-ref key (compared against `DAT_00abd480`
    = the "empty/sentinel" cell value; `word [ESI+0xc]` vs `[0x00abd480]`, `word [ESI+0xe]`
    vs `[0x00abd482]`).
- **Skip condition:** if the slot's `+0xc/+0xe` coord equals the sentinel `DAT_00abd480`
  (i.e. slot is empty), the slot is skipped (`JZ 0x0056bc45`). Only **occupied / pending**
  slots are processed.
- **Elapsed test** (for an occupied slot):
  - if `slot+0x0 == -1` -> use `remaining = slot+0x8` directly; elapsed iff `remaining == 0`.
  - else `elapsed_frames = g_CurrentFrameCounter(0x00a8ed84) - slot[0x0]`; if
    `elapsed_frames < remaining` -> not yet due (`remaining -= elapsed_frames`, then the
    `TEST ECX,ECX; JNZ` keeps it pending); otherwise it's due.
- **On due** (`0056bc37`–`0056bc40`), two calls in fixed order:
  1. `MOV ECX,ESI; CALL 0x004a1750`  = `CrateSlot__ClearAndPreserveTimer(slot)` — **no RNG**.
  2. `MOV ECX,EDI; CALL 0x0056bd40`  = `MapClass__PlaceCrateAtRandomCell(MapClass*)` —
     **draws RNG** (this is the consumer; see RNG section).
- **Note on spine callee labels:** spine named the second callee
  `MapClass__PlaceCrateAtRandomCell` — correct; its actual address is **`0x0056bd40`**
  (the spine's `0055b65a` was the parent CALL site, not this callee).

## Gate / mode condition

`disassemble_function 0x0056BBE0`, entry `0056bbe0`–`0056bbf3`:

```
0056bbe0  MOV  EAX,[0x00a8b238]      ; g_GameMode
0056bbe6  TEST EAX,EAX
0056bbea  JZ   0x0056bc4d            ; bail if game-mode == 0
0056bbec  MOV  AL,[0x00a8b261]       ; crates-enabled game option byte
0056bbf1  TEST AL,AL
0056bbf3  JZ   0x0056bc4d            ; bail if crates disabled
```

- **Confirmed gate (matches spine):** `g_GameMode (0x00a8b238) != 0` **AND**
  `DAT_00a8b261 != 0`.
- **`0x00a8b238` identity = g_GameMode:** same global the AnimClass rung (Rung U) tests at
  `0055b61b`–`0055b627` with `!= 0 && != 5`; the crate rung uses only the `!= 0` half. Heavy
  xref set across `Main_Game`, `Main_Tick`, `Network_ServiceLoop`, `State_Machine`,
  `Init_Random_Number_System` (`get_xrefs_to 0x00a8b238`) is consistent with the game-state /
  game-mode global. (Read of live image returns 0 because no game is loaded — static-image
  artifact, not a finding.)
- **`DAT_00a8b261` identity = crates-enabled option:** written by `ScenarioClass__Post_Map_Init`
  (`0068695c`) and `CDFileClass__Constructor`; read by `CrateClass__PickupDispatch`
  (`00481da5`), `Build_Options_String` (`005dbc1e`), and this driver (`get_xrefs_to
  0x00a8b261`). This is the "Crates" skirmish/MP setup option flag.

## RNG

- **Driver body itself draws NO RNG** — `CrateSlot__ClearAndPreserveTimer` (`0x004a1750`,
  `disassemble_function 0x004a1750`) only removes the crate overlay and rewrites the slot's
  cell-ref/timer fields (`CrateSlot__RemoveCrateOverlayFromCell` + arithmetic). No
  `Random__*` call.
- **RNG consumer = `MapClass__PlaceCrateAtRandomCell` @ `0x0056bd40`**
  (`decompile_function 0x0056bd40`, `disassemble_function 0x0056bd40`):
  - Finds the first **empty** slot (scan of `param_1+0x164` stride 0x10, up to 0x100). If none
    free (`EAX == 0x100`), returns 0 with **zero** RNG draws.
  - If a free slot exists, runs a **retry loop up to 1000 iterations** (`CMP EBX,0x3e8;
    JL`). **Each iteration draws exactly TWO `Random__RandomRanged` calls:**
    1. X cell: `RandomRanged(0, DAT_0087f914 - 1)` then `+ DAT_0087f90c` (call `0056bd9f`).
    2. Y cell: `RandomRanged(0, DAT_0087f918 - 1)` then `+ DAT_0087f910` (call `0056bdc1`).
    (`0087f90c/0087f914` and `0087f910/0087f918` are the map random-placement origin/extent
    in X/Y.) It then validates the cell (passable-cell search via
    `FootClass__Find_Nearby_Passable_Cell`, deterministic, no RNG) and tries to place the
    overlay+timer (`CrateSlot__PlaceOverlayAndInitTimer`, `0x0056dc20` via the `0x87f7e8`
    receiver, no RNG). On success returns 1 and **stops drawing** (loop exits).
  - **Draw total per due-slot:** `2 * N` `RandomRanged` draws, where `N` = number of attempts
    until a valid cell is found (1 ≤ N ≤ 1000); minimum 2 on first-attempt success. A due slot
    with no free table entry draws 0.
- **Stream = Scen->Random** (the ScenarioClass embedded RandomClass), NOT g_MainRng /
  g_MapGenRng. **Verified at the draw sites** (`disassemble_function 0x0056bd40`):
  ```
  0056bd90  MOV ECX,[0x00a8b230]   ; g_ScenarioClass_Instance
  0056bd97  ADD ECX,0x218          ; ECX = Scen + 0x218  (Scenario RNG state)
  0056bd9f  CALL 0x0065c7e0        ; Random__RandomRanged  (X draw)
  0056bdaa  MOV ECX,[0x00a8b230]
  0056bdb0  ADD ECX,0x218
  0056bdc1  CALL 0x0065c7e0        ; Random__RandomRanged  (Y draw)
  ```
  - `0x00a8b230` = `g_ScenarioClass_Instance` (same base read all over the parent
    `LogicClassPerTickUpdateLiveVector`: `0055afcd`, `0055b017`, `0055b09d`, etc.).
  - `Scen + 0x218` is the ScenarioClass RNG member (250-word RandomClass) — matches the
    project RNG-routing truth ("Scen->Random" = receiver `[0x00a8b230]+0x218`).
  - `Random__RandomRanged @ 0x0065c7e0` confirmed a genuine consumer (`__thiscall`;
    `decompile_function 0x0065c7e0` shows it advances the RandomClass ring buffer at
    `this+0xc` and indices `this+4`/`this+8`).
- **Lockstep note:** because placement uses **Scen->Random** (the synchronized gameplay
  stream) and the per-attempt draw count is data-dependent (1–1000 attempts), the exact
  number of draws consumed each tick is sensitive to map cell occupancy. This is part of the
  deterministic contract: every client must reach the same cell occupancy and therefore the
  same attempt count, or the Scenario RNG desyncs from this point forward.

## Active in YR / TS-legacy

- **Active in YR: CONDITIONAL — yes when "Crates" is enabled.** Not TS-legacy; crates are a
  standard, fully-live RA2/YR feature (the random goodie/money/unit/firestorm crates that
  appear on the battlefield).
- **Gate dependency:** the rung is a no-op unless `g_GameMode != 0` (an actual in-game
  scenario, not the menu/shell mode) **AND** `DAT_00a8b261 != 0` (the Crates game option,
  selectable in skirmish/MP setup). With Crates off it never walks. With Crates on (a common
  setting), elapsed timers respawn crates at random cells — directly player-visible (a crate
  overlay appears on a cell, drawn from `[CrateImg]`/SHP overlay; pickup behavior via
  `CrateClass__PickupDispatch`).
- **Reachable + visible in a normal YR skirmish:** YES when Crates is checked. The respawn
  cadence is governed by `[General] CrateRegen`/`CrateMinimum`/`CrateMaximum` (rules) feeding
  the per-slot timers seeded by `CrateSlot__PlaceOverlayAndInitTimer`.

---

## Verification calls (inline)

- `decompile_function 0x0055AFB0`, `disassemble_function 0x0055AFB0` — body site
  `0055b650`–`0055b667`; receiver `ECX=0x0087f7e8` set at `0055b655`; driver CALL at
  `0055b65a` (= spine's "0055b65a"); order W(0055b650) -> X(0055b65a) -> Y(0055b65f).
- `decompile_function 0x0056BBE0`, `disassemble_function 0x0056BBE0` — driver: gate
  `[0x00a8b238]!=0 && [0x00a8b261]!=0` (`0056bbe0`–`0056bbf3`); 256-slot walk at `this+0x158`
  stride 0x10; due-path calls `0x004a1750` then `0x0056bd40`.
- `decompile_function 0x004a1750`, `disassemble_function 0x004a1750` —
  `CrateSlot__ClearAndPreserveTimer`: clears overlay + rewrites slot timer fields; **no RNG**.
- `decompile_function 0x0056bd40`, `disassemble_function 0x0056bd40` —
  `MapClass__PlaceCrateAtRandomCell`: 1000-attempt loop, two `RandomRanged` per attempt on
  `Scen+0x218`; passable-cell search + overlay placement (no RNG).
- `disassemble_function 0x0056bd40` lines `0056bd90`/`0056bd97`/`0056bd9f`,
  `0056bdaa`/`0056bdb0`/`0056bdc1` — RNG receiver `[0x00a8b230]+0x218` = Scen->Random.
- `get_function_by_address 0x0065c7e0` + `decompile_function 0x0065c7e0` —
  `Random__RandomRanged` is `__thiscall`, advances RandomClass internal ring state (genuine
  consumer).
- `get_xrefs_to 0x00a8b230` — confirms it is g_ScenarioClass_Instance (refs in
  `Init_Random_Number_System`, `Main_Tick`, `Main_Game`, parent LogicClass).
- `get_xrefs_to 0x00a8b238` — game-mode global (also the AnimClass `!=0 && !=5` gate at
  `0055b61b`–`0055b627`).
- `get_xrefs_to 0x00a8b261` — crates-enabled option byte: written by
  `ScenarioClass__Post_Map_Init` (`0068695c`); read by `CrateClass__PickupDispatch`
  (`00481da5`), `Build_Options_String` (`005dbc1e`).
