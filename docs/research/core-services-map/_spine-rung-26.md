# Spine Rung 26 — Z. FactoryClass tick (production / build progress)

**Driver:** `FactoryClass::AI` @ **0x004C9B20** (vtable `vtable_FactoryClass` @ 0x007E88D0, slot +0x5C / index 23)
**Body site:** `LogicClassPerTickUpdateLiveVector` @ 0x0055AFB0, loop at **0x0055B66A–0x0055B68D**
**Order in ladder:** #26 (after Y. Tactical/DisplayClass tick @ 0x0055B65F-67; before AA. HouseClass tick @ 0x0055B68D-0x0055B6B3)

Status: **VERIFIED from binary.** Corroborated by existing `factory-house.md` (same address, same 54-step model).

---

## 1. Purpose (one line)

Walks the live FactoryClass vector and ticks each factory's pay-as-you-go production state machine — advancing the build bar one step per timer expiry, charging the per-step cost, stalling (OnHold) when the owning house can't afford the next step, and finishing the build at step 54.

## 2. What it walks / does

**Body loop** (verified via `disassemble_function 0x0055AFB0`):

```
0055b66a: MOV EAX,[0x00a83e40]            ; count = g_FactoryClass_Array_Count
0055b66f: XOR ESI,ESI                     ; i = 0
0055b671: TEST EAX,EAX
0055b673: JLE 0x0055b68d                  ; GATE: count > 0
0055b675: MOV ECX,dword ptr [0x00a83e34]  ; base = g_FactoryClass_Array
0055b67b: MOV ECX,dword ptr [ECX+ESI*0x4] ; entry = array[i]   (FactoryClass*)
0055b67e: MOV EDX,dword ptr [ECX]         ; vtable
0055b680: CALL dword ptr [EDX+0x5c]       ; FactoryClass::AI  (= 0x004C9B20)
0055b683: MOV EAX,[0x00a83e40]            ; reload count (re-read each iter)
0055b688: INC ESI
0055b689: CMP ESI,EAX
0055b68b: JL  0x0055b675                  ; FORWARD walk, ascending index
```

- Array base `0x00A83E34` = `g_FactoryClass_Array` (DynamicVector of `FactoryClass*`), count `0x00A83E40` = `g_FactoryClass_Array_Count`. Confirmed labels via `list_globals` and registration in `FactoryClass__Constructor 0x004C9983` (writes `*(g_FactoryClass_Array + count*4) = this; count++`). Verified via `get_xrefs_to 0x00a83e34`.
- **Forward (ascending) iteration**, NOT a reverse walk. Count is re-read every iteration (factories don't normally register/unregister inside their own AI, but the re-read makes the loop bound dynamic).

**Per-factory `FactoryClass::AI` 0x004C9B20** (verified via `decompile_function`/`disassemble_function 0x004c9b20`):

1. Early-out if `IsSuspended` (+0x70) set.
2. Early-out if nothing in production (`Object` +0x58 == 0 AND `SpecialItem` +0x68 == 0), or if already complete (`Production_Value` +0x24 == 0x36 with an Object present).
3. Read step timer remaining via `CDTimerClass__GetTimeRemaining 0x00426630` (CDTimer block at +0x2C). If still counting (≠0) or per-step duration (+0x38) is 0 → clear `Production_HasChanged` (+0x28) and return (no step this tick).
4. On timer expiry: advance `Production_Value += Production_Step` (+0x3C, =1 in YR), set HasChanged/IsDifferent, restart the CDTimer with `Production_Timer_Duration` (+0x38), and compute the per-step charge.
5. **Per-step charge** = `⌊Balance/(0x36 − Production_Value)⌋` (or full `Balance` when remaining-steps hits the `==0` guard), clamped to `Balance` (+0x60).
6. Query available funds: `(**(Owner+0x24 vtable +0x18))(Owner+0x24)` — the owning house's StorageClass purse total. If funds < charge → set `OnHold` (+0x5C), rewind `Production_Value -= 1` (no payment). Else → `HouseClass__Spend_Money 0x004F9790`, clear OnHold, `Balance -= charge`.
7. On reaching `Production_Value == 0x36` (54 = complete): set `IsSuspended`, zero the timer duration, settle the residual `Balance` via `Spend_Money`, zero `Balance`.

Net observable: the 0→54 build-bar fill, the "hold / insufficient funds" stall, and pay-as-you-go credit drain on the sidebar.

## 3. Gate / mode condition

**Body-site gate: `g_FactoryClass_Array_Count > 0`** (unconditional otherwise). CONFIRMED — matches the spec.
- **No `g_GameMode` gate** here (unlike rung U AnimClass `g_GameMode != 0 && != 5`, or rung X crate-regen). The factory walk runs in every mode where factories are registered.
- Per-factory internal gates inside `FactoryClass::AI`: `IsSuspended`, empty-queue, already-complete, and the CDTimer step-rate throttle (build only advances when the per-step timer elapses). These are *internal* short-circuits, not the ladder gate.

## 4. RNG draws

**NONE.** No RNG draw on this rung — verified.
- `FactoryClass::AI` callees (`get_function_callees 0x004c9b20`): only `CDTimerClass__GetTimeRemaining 0x00426630` (timer read) and `HouseClass__Spend_Money 0x004F9790` (×2). Plus one indirect call `[Owner+0x24 vtable+0x18]` = StorageClass funds getter.
- Disassembly (`disassemble_function 0x004c9b20`) shows zero `CALL` to any RNG routine; arithmetic is plain integer `IDIV`/`SUB` for the per-step cost (`⌊Balance/(54−Value)⌋`), no `Scen->Random` / `g_MainRng` / `g_MapGenRng` receiver loaded anywhere.
- `HouseClass__Spend_Money 0x004F9790` (verified via `decompile_function`) is pure StorageClass amount bookkeeping + credit-state notify (`Math__ftol`, `StorageClass__*`) — no RNG.

**rng_stream: none. draws_rng: false. rng_notes: none.**

## 5. Active-in-YR / Tiberian Sun legacy

**ACTIVE in YR — core, every-match, player-visible.** NOT TS legacy.
- FactoryClass instances are registered for every active production queue (construction yard, war factory, barracks, naval yard, etc.). `g_FactoryClass_Array_Count > 0` holds in essentially every normal skirmish the moment a player queues anything.
- Observable output every match: sidebar cameo build-bar fill cadence, "On Hold" stall when credits run dry mid-build, incremental credit drain (pay-as-you-go), and build completion → ready flash. These are exactly the parity-bar-listed observables (sidebar flash/cameo cadence).
- The step-rate (CDTimer duration +0x38) derives from `GetBuildStepTime`/54 (RecalcAllRates, see `factory-house.md`), i.e. build speed is faithfully throttled here, not approximated.

## 6. Evidence (Ghidra calls)

- `decompile_function 0x0055AFB0` — located the rung loop and confirmed neighbors (Tactical @ 0x0055B65F before, HouseClass @ 0x0055B68D after).
- `disassemble_function 0x0055AFB0` — exact body loop bytes 0x0055B66A–0x0055B68D; confirmed forward walk, base 0x00A83E34, count 0x00A83E40, vt+0x5C indirect call.
- `list_globals 00a83e34 / 00a83e40` — `g_FactoryClass_Array` / `g_FactoryClass_Array_Count`.
- `get_xrefs_to 0x00a83e34` — registration site `FactoryClass__Constructor 0x004C9983`.
- `decompile_function 0x004C9983` — primary vtable = `vtable_FactoryClass`; registration push into the array.
- `list_globals vtable_FactoryClass` → 0x007E88D0; `read_memory 0x007E892C` (vt+0x5C) → bytes `20 9b 4c 00` = **0x004C9B20**.
- `decompile_function 0x004C9B20` + `disassemble_function 0x004C9B20` — 54-step state machine, per-step charge, OnHold rewind, completion settle; no RNG.
- `get_function_callees 0x004C9B20` — only CDTimer-GetTimeRemaining + Spend_Money (+ indirect funds getter).
- `decompile_function 0x004F9790` — Spend_Money is StorageClass bookkeeping, no RNG.
- Cross-check: existing `docs/research/core-services-map/factory-house.md` lines 13, 20, 45 confirm the same address and 54-step model.
