# Spine Rung 27 — AA. HouseClass tick (economy / power / SW / AI; null-checked)

> **2026-08-29 correction:** defeat's `0x004FC6D0` call is a destructive live
> Techno/C4 receiver sweep, not movement Scatter. The corrected mechanism is
> `docs/gap-scans/2026-08-29-disparity-scan-action-119-house-destruction.md`.

**Driver:** `HouseClass__Update` (a.k.a. HouseClass::AI) @ **0x004F8440** (vtable `vtable__HouseClass` @ 0x007EA8A0, slot +0x5C / index 23)
**Body site:** `LogicClassPerTickUpdateLiveVector` @ 0x0055AFB0, loop at **0x0055B68D–0x0055B6B3**
**Order in ladder:** #27 (after Z. FactoryClass tick @ 0x0055B66A-8D; before AB. Last-ref camera/audio follow + temp teardown @ 0x0055B6B3+)

Status: **VERIFIED from binary.**

---

## 1. Purpose (one line)

Walks every registered HouseClass (player/faction) and ticks its per-house bookkeeping: power assess/recheck, super-weapon ready poll, low-power EVA, ally-vision/persistence timers, defeat detection (destructive House Techno sweep + MPlayer_Defeated), the AI build/unit/aircraft/infantry choosers + production management, and per-player EVA cues (insufficient funds / silos needed).

## 2. What it walks / does

**Body loop** (verified via `disassemble_function 0x0055AFB0`):

```
0055b68d: MOV EAX,[0x00a80238]            ; count = g_HouseClass_Array_Count
0055b692: XOR ESI,ESI                     ; i = 0
0055b694: TEST EAX,EAX
0055b696: JLE 0x0055b6b3                  ; GATE: count > 0
0055b698: MOV EAX,[0x00a8022c]            ; base = g_HouseClass_Array
0055b69d: MOV ECX,dword ptr [EAX+ESI*0x4] ; entry = array[i]   (HouseClass*)
0055b6a0: TEST ECX,ECX
0055b6a2: JZ  0x0055b6a9                  ; PER-ENTRY null-check: skip null slot
0055b6a4: MOV EDX,dword ptr [ECX]         ; vtable
0055b6a6: CALL dword ptr [EDX+0x5c]       ; HouseClass::AI  (= 0x004F8440)
0055b6a9: MOV EAX,[0x00a80238]            ; reload count (re-read each iter)
0055b6ae: INC ESI
0055b6af: CMP ESI,EAX
0055b6b1: JL  0x0055b698                  ; FORWARD walk, ascending index
```

- Array base `0x00A8022C` = `g_HouseClass_Array` (DynamicVector of `HouseClass*`), count `0x00A80238` = `g_HouseClass_Array_Count`. **Note: the decompiler renders these two reads as `g_HouseClass_Array` / `g_HouseClass_Array_Count` in this function, which is correct in meaning** — the spine spec's `DAT_00a8022c` / `DAT_00a80238` are the same addresses. Confirmed via `get_xrefs_to 0x00a8022c` and `get_xrefs_to 0x00a80238`: registration site is `HouseClass__Constructor` (writes `*(g_HouseClass_Array + count*4)=this; count++` at 0x004F61E0/0x004F61E6); consumers include `HouseClass__Recalculate_Alliances`, `HouseClass__Is_Enemy`, `HouseClass__MPlayer_Defeated`, `HouseClass__Find_By_Country_Index`, `HouseClass__FindByName` — unambiguously the global house array.
- **Forward (ascending) iteration**, with a **per-entry non-null guard** (rungs T/U/etc. do not null-check; this one does — matches the spec "per-entry non-null"). Count re-read every iteration.

**Per-house `HouseClass__Update` 0x004F8440** (verified via `decompile_function`/`disassemble_function 0x004f8440`) — large per-tick function, key stages:

1. Power/radar recheck timers (+0x2A4/+0x2B0): on expiry set `RecheckPower`/`RecheckRadar`.
2. If `RecheckPower`: `HouseClass__AI_AssessPower 0x00508C30`, force RecheckRadar.
3. If `RecheckRadar`: `HouseClass__CheckSuperweaponReady 0x00508DF0` (gated `DAT_00a8b538==0`) + `HouseClass__CheckLowPower 0x00508F60`.
4. Every 100 frames: decrement per-house "build limit" counters (+0x5608 vector).
5. Radar-up / radar-down / scatter notification timers (+0x298 family) — fire EVA flags `DAT_00a83d49`/`DAT_00a8ecd0`, pumping `VoxClass__PumpAndCheckActive 0x007529e0` + `Network_ServiceLoop 0x0048d080` while the radar anim plays.
6. **Local-player force-fire-sale / paranoid-placement RNG block** (`0x004f8838–0x004f890d`) — see §4. Draws RNG to pick a cell occupant to harass; local-player + network only.
7. Per-15-frame "attacked-cell" auto-response: `MapClass__Get_CellClass` → `CellClass__Find_Nearest_Object` → up to 5 retaliation fire attempts via object vt+0x16C.
8. Build-rate / unit-rate timers, defeat detection: when owned buildings+units hit zero (and not observer), the destructive House Techno sweep at `0x004FC6D0` + `HouseClass__MPlayer_Defeated`.
9. SuperClass AI-ready poll over the house's super-weapon vector (+0x258), flashing the sidebar tab for the local player.
10. Every 8 frames (`& 0x80000007`): AI build/unit/aircraft/infantry choosers (`HouseClass__AI_Choose_*` 0x004FE3E0/0x004FEA60/0x004FEEE0/0x004FF210) + `HouseClass__AI_Building_Strategy 0x004FD500`, gated by non-current-player / non-civilian-AI house.
11. Production management for the queue-dirty flag (+0x1FC): `HouseClass__AI_ManageProduction 0x0050AF10` + `AI_ResumeProduction 0x0050B1D0`.
12. Local-player-only EVA cues: insufficient-funds warning (`StringTable__LoadString` 0x949 → `FUN_005d3ba0`), silos-needed cue, plus a final `MapClass__ParanoidUnrevealAll` when flag +0x24B set.

## 3. Gate / mode condition

**Body-site gate: `g_HouseClass_Array_Count > 0` AND each entry non-null.** CONFIRMED — matches the spec exactly (count>0 + per-entry null-check at 0x0055b6a0). No `g_GameMode` gate at the body site.

Inside `HouseClass__Update`, most heavy AI/defeat stages are internally gated by a recurring "is this the controllable/current player?" predicate (read `CurrentPlayer` +0x1EC / `PlayerControl` +0x1ED, forced true in `g_GameMode==0` campaign) and by `Type[0x1A6]` (civilian/special house). These are *internal* short-circuits, not the ladder gate — the rung itself ticks every registered house unconditionally.

## 4. RNG draws

**YES — up to 3 draws, but only on the LOCAL player's house in a network game, and routed to TWO different streams.** The decompiler labels the draw helper `Random__RandomRanged 0x0065C7E0`; this helper is `__thiscall` and operates on the RNG instance passed in **ECX** (param_1). The instance is therefore per-callsite — verified by reading ECX at each draw in `disassemble_function 0x004f8440`:

**Draw block `0x004f883e–0x004f890d`** — entire block gated by:
- `0x004f8830-38`: `*(int*)([0x00a83d4c]+0x30) == *(int*)(this+0x30)` → this house's ArrayIndex equals the **local/current player house** (`0x00A83D4C`, written by `Main_Game 0x0052D9DE`, read by `HouseClass__IsHumanPlayer`).
- `0x004f883e-4b`: `g_GameMode (0x00A8B238) == 3 || == 4` → network/LAN/internet multiplayer only.
- `0x004f8851-66`: spectator/observer manager `0x00A8B23C` non-null AND its vt+0x4 returns 0 (not spectating).

Draw sites within the block:
1. `0x004f887d: MOV ECX,0x886b88` → `0x004f8888: CALL 0x0065C7E0` with args `(0,1)` → result stored to `DAT_00a8efe8`. Receiver `0x00886B88` = **g_MainRng** (verified: `Init_Random_Number_System 0x0052FE00` seeds `&g_MainRng` at 0x00886B88 alongside `Scen->Random` at Scenario+0x218). One-time init (guarded by `DAT_00a8f03c & 1`).
2. `0x004f8895: MOV ECX,0x886b88` → `0x004f889A: CALL 0x0065C7E0` with args `(0,2)`. Receiver `0x00886B88` = **g_MainRng**. Used to decide whether to harass a cell (`if==0`).
3. `0x004f8902: LEA ECX,[EAX+0x218]` where EAX=`g_ScenarioClass_Instance (0x00A8B230)` → `0x004f8908: CALL 0x0065C7E0` with args `(0,2)`. Receiver `Scenario+0x218` = **Scen->Random**. Conditional — only when the picked cell holds a live occupant (`*(int*)(cell+0x130)!=1 || cell flags+0x12C & 0x18`). Result discarded.

**rng_stream: Scen->Random AND g_MainRng (mixed, per-draw).** **draws_rng: true.** **rng_notes:** up to 2 g_MainRng draws (init `(0,1)`→g_MainRng once; per-tick `(0,2)`) and up to 1 Scen->Random `(0,2)` draw — ALL gated behind local-player + network(mode 3/4) + not-spectating + a per-tick mode-8 cell check. The g_MainRng draws are non-lockstep (UI stream). The single Scen->Random draw is local-player-gated, so it cannot be a synchronized-sim draw (it would desync if it were) — this is a cosmetic/EVA "harass an occupied cell" flourish, not core lockstep state.

> Caveat for the lockstep-draw-order contract: because all three draws are *local-player-gated*, this rung consumes **0 synchronized Scen->Random draws** on non-local houses. On the local house in a mode-3/4 game it may consume 1 Scen->Random draw per relevant tick. (`HouseClass__Constructor` itself draws Scen->Random via `Random__RandomRanged(0x1C2,0x708)` for the build-delay jitter, but that is construction, not this per-tick rung.)

## 5. Active-in-YR / Tiberian Sun legacy

**ACTIVE in YR — core, every-match.** NOT TS legacy.
- HouseClass instances are registered for every player and AI faction; `g_HouseClass_Array_Count > 0` always holds in any skirmish.
- Player-visible outputs every match: low-power EVA, super-weapon ready flash, insufficient-funds / silos-needed EVA cues, AI opponents building and producing, and defeat handling (the live-Techno destruction/C4 sweep followed by "player defeated"). These are exactly the kind of outputs the parity bar covers.
- The RNG "harass an occupied cell" sub-block (§4) is a narrow local-player network-only path (gamemode 3/4, not-spectating). It is reachable in a normal online/LAN YR skirmish for the local human, but is a minor cosmetic flourish; it does **not** fire in campaign (mode 0) or for AI/remote houses, and its draws do not change synchronized state.

## 6. Evidence (Ghidra calls)

- `decompile_function 0x0055AFB0` — located the rung-27 loop and confirmed neighbors (FactoryClass @ 0x0055B66A before, last-ref follow @ 0x0055B6B3 after).
- `disassemble_function 0x0055AFB0` — exact body loop bytes 0x0055B68D–0x0055B6B3: forward walk, base `0x00A8022C`, count `0x00A80238`, **per-entry TEST ECX,ECX null-guard**, vt+0x5C indirect call.
- `get_xrefs_to 0x00a8022c` / `get_xrefs_to 0x00a80238` — registration via `HouseClass__Constructor` (write at 0x004F61E0); consumers `HouseClass__Recalculate_Alliances`, `HouseClass__Is_Enemy`, `HouseClass__MPlayer_Defeated`, etc. → confirms global HouseClass array.
- `decompile_function 0x004F5F00` (HouseClass__Constructor) — `*param_1 = &vtable__HouseClass`; array push at 0x004F61E6.
- `read_memory 0x007EA8A0` (112 bytes) — vtable dump; slot +0x5C (byte offset 92) = `40 84 4f 00` = **0x004F8440**. (vtable base 0x007EA8A0 corroborated by docs/research/HOUSE_RESULT_BYTES…/LABEL_AUDIT_LOG.)
- `read_memory 0x007EA8FC` — vt+0x5C = `0x004F8440`; `get_xrefs_to 0x004F8440` → only `0x007EA8FC [DATA]` (pure virtual dispatch, no direct callers).
- `decompile_function 0x004F8440` + `disassemble_function 0x004F8440` — full per-house AI/economy/power/SW stage list; located the 3 RNG draw sites and their ECX receivers.
- `decompile_function 0x0065C7E0` — `Random__RandomRanged` is `__thiscall`, draws from the RNG object in param_1 (ECX) → instance is per-callsite.
- `decompile_function 0x0052FE00` (Init_Random_Number_System) — seeds `Scenario+0x218` = **Scen->Random** and `&g_MainRng` = **0x00886B88** = g_MainRng. Resolves the two receivers.
- `get_xrefs_to 0x00886B88` — written only by `Init_Random_Number_System`; read at HouseClass__Update 0x004F887D/0x004F8895 → confirms g_MainRng instance.
- `get_xrefs_to 0x00a83d4c` — written by `Main_Game 0x0052D9DE`, read by `HouseClass__IsHumanPlayer` → local/current player house pointer (gates the RNG block).
- `get_xrefs_to 0x00a8b23c` — read by `Main_Tick` / network reveal code → spectator/observer manager (network-game presence).
