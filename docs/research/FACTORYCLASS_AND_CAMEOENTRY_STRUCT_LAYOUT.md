# FactoryClass + CameoEntry Struct Layout — Canonical Reference

**Addresses:** FactoryClass at `0x007E88D0` (vtable), constructor entry at `0x004C98B0`.
StripClass::CameoEntry array embedded at strip + 0x58, stride 0x34.
**Confidence:** HIGH for the corrected offsets/mechanics re-verified live on
2026-07-11; the explicitly marked deferred/UNVERIFIED semantics remain open.
**Active in YR:** Yes — core sidebar/production system, runs every tick.

**2026-07-11 correction:** the earlier `0x004C98F0` constructor citation was an
interior address; the live function entry is `0x004C98B0` (`get_function_by_address
0x004C98B0` and `0x004C98F0`). Root cause: `GHIDRA_ADDRESS_SHIFT`.

This report consolidates and corrects field-layout claims that were
scattered across BUILD_QUEUE_GHIDRA_REPORT.md, SIDEBAR_STRIPS_TABS_CAMEOS_GHIDRA.md,
SIDEBAR_SYSTEM_GHIDRA_REPORT.md, FACTORYCLASS_PRODUCTION_DEEP_DIVE.md,
and FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md. The earlier docs had:

- BUILD_QUEUE's `Rate (+0x38)` is correct: it is separate from the embedded
  timer duration at `+0x34`; the prior version of this document incorrectly
  removed that rate field (`disassemble_function 0x004C9B20`,
  `0x004C9E60`, `0x004C9EA0`, and `0x004CA6E0`). Root cause:
  `STRUCT_FAMILY_CASCADE` from treating the rate as part of `CDTimerClass`.
- SIDEBAR_SYSTEM labelling CameoEntry +0x1C..+0x30 as flash fields (the
  embedded CDTimerClass is only +0x1C..+0x24; +0x28 is a separate rate,
  +0x2C is the step, and +0x30 is the flash deadline; verified by
  `disassemble_function 0x006A8B30` and `0x006A8710`). Root cause:
  `STRUCT_FAMILY_CASCADE`.
- SIDEBAR_STRIPS labelling CameoEntry +0x28 as FlashEndFrame and +0x30 as FlashTimer (swapped — +0x30 is the actual flash deadline)
- Mixed-up claims about progress range (0..0x34 vs 0..0x36)

The truth turns out to be: **there are two separate progress fields**, in
two separate structs, with separate termination rules and separate roles.

---

## 1. Overview

`FactoryClass` is the per-house, per-category production state machine
(one factory per production tab × per house, maximum). It owns the
TechnoClass being built (`Object` at +0x58), a `Production_Value`
counter that drives the cost-deduction and completion logic, and a
`DynamicVectorClass<TechnoTypeClass*>` queue.

`CameoEntry` is a 0x34-byte slot embedded in `StripClass` (75 per strip,
4 strips per sidebar). It carries an auxiliary progress value, a local
timer/rate pair, and the per-cameo flash deadline. `StripClass::Draw`
starts from `FactoryClass::GetProgress`; when the cameo value is larger,
it draws the integer midpoint of the factory and cameo values. The cameo
field is therefore not an independent replacement for factory progress
(`decompile_function 0x006A9540`, branch around `LAB_006a98c4`). Root
cause of the prior description: `INFERENCE_HARDENED`.

The two progress fields are linked but not equal. Treat them as
**two separate state variables**:

| Field | Lives in | Limit / threshold | Role |
|-------|----------|------|------|
| `FactoryClass.Production_Value` | FactoryClass +0x24 | **0x36 = 54** | Drives cost deduction (one of 54 cost steps), gates completion (`IsComplete` checks `== 0x36`), survives save/load |
| `CameoEntry.ProgressValue` | CameoEntry +0x14 | stop threshold **>= 0x35** after increment | Auxiliary upper smoothing value. Draw uses factory progress unless this value is larger, then uses `(factory + cameo) / 2`; the final shape frame argument is that result + 1 (`disassemble_function 0x006A8B30` at `0x006A9000..0x006A9017`; `decompile_function 0x006A9540`). Root causes: `OPERATOR_OR_ORDER_DRIFT`, `INFERENCE_HARDENED`. |

---

## 2. FactoryClass — Canonical Field Layout (0x74 bytes total)

Verified via `decompile_function 0x004C98B0` (constructor), `0x004CA430`
(debug dump), `0x004C9B20` (AI tick), `0x004CA130` (IsComplete),
`0x004C9FF0` (AbandonProduction), `0x004CA120` (GetProgress, raw bytes
`8b 41 24 c3` = `mov eax, [ecx+0x24]; ret`), all on 2026-05-20.

| Offset | Size | Field | Constructor init | Notes |
|--------|------|-------|------------------|-------|
| +0x00 | 4 | vtable_FactoryClass | `0x007E88D0` | Primary vtable. AI is at slot 23 (vtable+0x5C). |
| +0x04 | 4 | secondary_vtable_4 | secondary IUnknown/IRTTI | AbstractClass embeds at +0x04. `AbstractClass__AssignUniqueID(param_1 + 1)` writes ID here. |
| +0x08 | 4 | secondary_vtable_8 | secondary IPersist | |
| +0x0C | 4 | secondary_vtable_12 | secondary IPersistStream | Primary-vtable Load entry is 0x004CA270; see §2.1. |
| +0x10..+0x23 | 0x14 | (AbstractClass body) | (inherited init) | Includes the AbstractClass UID/Owner block; details out of scope for this doc. |
| **+0x24** | **4** | **Production_Value** | **0** | int, range 0..0x36. The canonical production-progress counter. Incremented by `Production_Step` (= 1) each `Production_Timer` expiry inside `FactoryClass::AI`. Completion: `== 0x36`. **NOT a percent — it is a literal step counter.** |
| +0x28 | 1 | Production_HasChanged | 0 | byte. Set false on the AI no-advance return and true on an advance. It is **not** read by `FactoryClass::HasChanged`; that method reads-and-clears +0x5D (`disassemble_function 0x004C9B20` and `0x004C9C60`). Root cause: `OFFSET_RETYPED_WRONG`. |
| +0x29..+0x2B | 3 | (padding) | — | |
| +0x2C | 4 | Production_Timer.StartTime | `g_CurrentFrameCounter` | int. Frame at which the current step's timer started. |
| +0x30 | 4 | Production_Timer.pad | (uninitialised in constructor — picks up whatever was in `local_8`) | int. Unused/reserved field within the embedded CDTimerClass. Constructor writes `local_8` here from an uninitialised stack slot (Ghidra decomp shows `param_1[0xc] = local_8;` with no prior write to `local_8`). Treat as scratch. |
| +0x34 | 4 | **Production_Timer.Duration** | 0 | int. Third and final dword of the embedded `CDTimerClass` at +0x2C. `CDTimerClass__GetTimeRemaining` reads +0x34 and computes remaining time as `Duration - (CurrentFrame - StartTime)`; there is no stored `TimeLeft` dword (`decompile_function` and `disassemble_function 0x00426630`). Root cause: `STRUCT_FAMILY_CASCADE`. |
| +0x38 | 4 | **Production_Rate** | 0 | int. Separate per-step interval/rate. AI tests it after `GetTimeRemaining`; on expiry AI copies it into timer `Duration (+0x34)`. `SetRate` writes both +0x38 and +0x34; `RecalcAllRates` updates only +0x38 (`disassemble_function 0x004C9B20`, `0x004C9EA0`, `0x004CA6E0`). Root cause: `STRUCT_FAMILY_CASCADE`. |
| +0x3C | 4 | Production_Step | 1 | int. Per-tick increment for `Production_Value`. **Always 1 in stock YR.** No code path writes anything else. |
| +0x40 | 4 | QueuedObjects.vtable | `0x007E8934` | DynamicVectorClass<TechnoTypeClass*>. |
| +0x44 | 4 | QueuedObjects.ArrayPtr | 0 | Heap-allocated array of TechnoTypeClass*. |
| +0x48 | 4 | QueuedObjects.Capacity | 0 | Allocated slot count. |
| +0x4C | 1 | QueuedObjects.IsAllocated | 1 | byte. Constructor sets to 1 — vector owns its array. |
| +0x4D | 1 | QueuedObjects.IsInitialized | 0 | byte. |
| +0x4E..+0x4F | 2 | (padding) | — | |
| +0x50 | 4 | QueuedObjects.Count | 0 | int. Live queue length. Doc cap = `RulesClass.MaximumQueuedObjects` (RulesClass +0xF0). |
| +0x54 | 4 | QueuedObjects.GrowthIncr | 10 | int. Vector reallocates in increments of 10. |
| +0x58 | 4 | Object | 0 | `TechnoClass *`. The currently-active produced object (set by `StartProduction`, cleared by `CompletedProduction`/`AbandonProduction`). |
| +0x5C | 1 | OnHold | 0 | byte. **NoFunds flag** — set to `true` in `FactoryClass::AI` when the per-step cost exceeded the house's available credits; cleared on the next tick when credits return. Read by `FUN_004C9C50` (a getter, returns `*(byte *)(this + 0x5C)`). Distinct from `IsSuspended` — `OnHold` is "system stalled on funds"; `IsSuspended` is "production paused" (either user or system). |
| +0x5D | 1 | IsDifferent | 0 | byte. Set by AI when state changes; consumed by the sidebar refresh. (Sometimes called `Production_HasChanged` in old docs — different from the +0x28 byte; +0x28 is per-tick AI-internal, +0x5D is "needs UI refresh".) |
| +0x5E..+0x5F | 2 | (padding) | — | |
| +0x60 | 4 | Balance | 0 | int. Remaining cost owed for the current `Object`. Decremented by per-step `costThisStep` each AI tick. Reaches 0 when production completes. |
| +0x64 | 4 | OriginalBalance (debug/CRC label) | 0 | The constructor zeros it and the debug/CRC method reads it, but `StartProduction` does not write it and `AbandonProduction` computes `type->GetCost() - Balance` directly (`disassemble_function 0x004C98B0`, `0x004C9C70`, `0x004C9FF0`, `0x004CA430`; `search_instructions MOV, operand 0x64]` finds no other FactoryClass writer). Its intended live role remains UNVERIFIED. Root cause of the old start/refund semantics: `INFERENCE_HARDENED`. |
| +0x68 | 4 | SpecialItem | -1 | int. Sentinel for non-Object production (e.g., superweapons). `0` and `-1` mean "no special item". Active when not -1 and not 0. |
| +0x6C | 4 | Owner | 0 | `HouseClass *`. The house that owns this factory. |
| +0x70 | 1 | IsSuspended | 0 | byte. **Production paused.** Set when: production completes (waits for placement), `Suspend()` called, `AbandonProduction()` ran, or `StartProduction()` initialised a fresh slot. |
| +0x71 | 1 | **IsManual** | **1 (default)** | byte. Pause-reason flag. Constructor initializes 1; `Suspend(bool)` copies its argument directly, and the user suspend command passes 1 (`disassemble_function 0x004C98B0`, `0x004C9E60`, `0x004FA910`). The prior `canAfford` parameter name was unsupported. Root cause: `PARAM1_TYPE_MISREAD`. |
| +0x72..+0x73 | 2 | (padding to round up to 0x74) | — | |

**Total size:** 0x74 bytes (`get_assembly_context 0x004CA760` shows the
virtual size method returning 0x74; `disassemble_function 0x004FA350` at
0x004FA4E5 pushes 0x74 before allocation). Root cause of the earlier
second-hand citation: `INFERENCE_HARDENED`.

### 2.1 Vtable layout (selected slots, from FACTORYCLASS_PRODUCTION_DEEP_DIVE)

The primary vtable is at `0x007E88D0`. Slot 23 (vtable+0x5C) is the per-tick `AI` method — `LogicClass::AI` walks `g_FactoryClass_Array` and calls vtable[23] for each.

| Slot offset | Address | Method |
|---|---|---|
| +0x14 | 0x004CA270 | Load (IPersistStream) |
| +0x18 | 0x004CA3C0 | Save (IPersistStream) |
| +0x20 | (Release) | Destructor |
| +0x5C | **0x004C9B20** | **AI (the production tick)** |

### 2.2 FactoryClass method table (re-verified from prior deep-dive)

| Address | Name | Notes |
|---|---|---|
| 0x004C98B0 | Constructor | Initializes/registers the already-allocated 0x74-byte object; allocation occurs at callers such as `HouseClass::Begin_Production` (`get_function_by_address 0x004C98B0`; `disassemble_function 0x004FA350`). Root causes: `GHIDRA_ADDRESS_SHIFT`, `OPERATOR_OR_ORDER_DRIFT`. |
| 0x004C9B20 | AI (vtable[23]) | Per-frame production tick (see §3.2). |
| 0x004C9C60 | HasChanged | Read-and-reset of `IsDifferent (+0x5D)`. |
| 0x004C9C70 | StartProduction | Fresh-start path resets Rate +0x38 and timer Duration +0x34 before creating the object; otherwise it queues subject to cap/build-limit/vector-growth logic (`decompile_function` and `disassemble_function 0x004C9C70`). Root cause of the old timer-field interpretation: `STRUCT_FAMILY_CASCADE`. |
| 0x004C9E60 | Suspend | If not already suspended, copy the boolean argument to IsManual, set IsSuspended, zero Rate +0x38, and reset the timer with Duration +0x34 = 0 (`disassemble_function 0x004C9E60`). Root causes: `PARAM1_TYPE_MISREAD`, `OPERATOR_OR_ORDER_DRIFT`. |
| 0x004C9EA0 | SetRate (Resume) | Compute rate, unsuspend, write Rate +0x38 and start timer with Duration +0x34 (`get_function_by_address 0x004C9EA0`; `disassemble_function 0x004C9EA0`). Root causes: `GHIDRA_ADDRESS_SHIFT`, `STRUCT_FAMILY_CASCADE`. |
| 0x004C9FB0 | CalcRate | Return `clamp((Object ? ObjectBuildTime : 0) / 0x36, 1, 0xFF)`; the full build-time provider is the call at 0x006F47A0 (`get_function_by_address`, `decompile_function`, and `disassemble_function 0x004C9FB0`). Root cause of the old name/role: `RTTI_LABEL_DRIFT`. |
| 0x004C9FF0 | AbandonProduction | Refund + reset + delete Object (`get_function_by_address 0x004CA0E0` resolves to entry 0x004C9FF0). Root cause: `GHIDRA_ADDRESS_SHIFT`. |
| 0x004CA120 | **GetProgress** | `return this->Production_Value;` (raw: `8b 41 24 c3` — verified live). Range **0..0x36**, NOT 0..100. |
| 0x004CA130 | **IsComplete** | Returns true iff `(Object != NULL && Production_Value == 0x36) \|\| (SpecialItem != -1 && Production_Value == 0x36)`. |
| 0x004CA160 | GetObject | `return this->Object;` |
| 0x004CA1A0 | CompletedProduction | On a complete Object (or qualifying SpecialItem), clear it, suspend, flag different, zero progress and Rate +0x38, and reset timer Duration +0x34 (`decompile_function 0x004CA1A0`). Root cause of the old timer-field interpretation: `STRUCT_FAMILY_CASCADE`. |
| 0x004CA5A0 | StartNextQueued | Only when queue nonempty, Object null, and `(Rate +0x38 == 0 || IsSuspended)`: remove the front entry, shift the queue, and invoke the house begin-production path if the type heap ID is nonnegative (`decompile_function` and `disassemble_function 0x004CA5A0`). Root causes: `STRUCT_FAMILY_CASCADE`, `OPERATOR_OR_ORDER_DRIFT`. |
| 0x004CA620 | RemoveFromQueue | Find/erase one type from queue. |
| 0x004CA670 | CountTypeInFactory | Count `Object + queued` matches of one TechnoTypeClass. |
| 0x004CA6B0 | IsInQueue | Membership predicate. |
| 0x004CA6E0 | RecalcAllRates | For each factory whose Owner +0x6C equals the supplied house, recompute/clamp the rate and update only +0x38 when changed; it does not call SetRate or restart +0x34 (`decompile_function` and `disassemble_function 0x004CA6E0`). Root cause: `OPERATOR_OR_ORDER_DRIFT`. |

---

## 3. The Two Progress Fields — How They Interact

This is the canonicalisation goal of this doc. **They are not the same
field.** They have separate caps, separate update sites, and separate
roles.

### 3.1 FactoryClass.Production_Value (+0x24, cap 0x36 = 54)

The authoritative production counter. Drives:

- **Per-step cost deduction** (`FactoryClass::AI`): on each timer expiry,
  `Production_Value += 1` and `costThisStep = Balance / (0x36 - Production_Value)`,
  clamped to `Balance`. If the house can't afford the step, `OnHold = true`
  and `Production_Value` is rolled back by 1 (net zero progress).
- **Completion gate** (`FactoryClass::IsComplete`): returns true iff
  `Production_Value == 0x36`.
- **Auto-suspend on completion**: `FactoryClass::AI` sets `IsSuspended = true`
  the moment `Production_Value == 0x36`. The factory then waits for placement
  (or queue advance).
- **Refund** (`FactoryClass::AbandonProduction`): `refund = type->GetCost() - Balance`.
  Production_Value isn't directly in the refund formula, but it influences `Balance`
  via the per-step deduction.
- **Save/load**: serialised via IPersistStream. `AbstractClass::Save` writes
  the full virtual size (0x74) raw, then `FactoryClass::Save` writes queue
  count/items separately; Load reconstructs vtables/vector storage and
  remaps queue, Owner, and Object pointers (`disassemble_function
  0x00410320`, `0x004CA3C0`, and `0x004CA270`).

The value is read by `FactoryClass::GetProgress` (0x004CA120) and exposed
to the UI through that getter — but the UI does **not** display this
value directly as a percent; the sidebar polls the factory then drives
its own `CameoEntry.ProgressValue` (see §3.3).

### 3.2 FactoryClass::AI — the canonical progress driver (0x004C9B20)

Verified via live decompile, 2026-05-20. The full guard structure:

```c
void FactoryClass::AI() {
    if (this->IsSuspended) return;
    if (this->Object == NULL && this->SpecialItem == 0) return;
    if (this->Object != NULL && this->Production_Value == 0x36) return;
    if (this->SpecialItem == -1 || this->Production_Value != 0x36) {
        int timeRemaining = CDTimerClass__GetTimeRemaining(&this->Production_Timer);
        if (timeRemaining != 0 || this->Production_Rate == 0) {
            this->Production_HasChanged = false;   // +0x28 (NOT +0x5D)
            return;
        }
        // === ADVANCE ONE STEP ===
        this->Production_Value += this->Production_Step;  // +1
        this->Production_HasChanged = true;               // +0x28
        this->Production_Timer.StartTime = g_CurrentFrameCounter;
        this->Production_Timer.Duration = this->Production_Rate;
        this->IsDifferent = true;                         // +0x5D
        // ... cost deduction ...
        if (this->Production_Value == 0x36) {
            this->IsSuspended = true;
            this->Production_Rate = 0;
            this->Production_Timer.Duration = 0;
            // ... reset timer start/pad ...
            HouseClass__Spend_Money(this->Balance);
            this->Balance = 0;
        }
    }
}
```

**Tiny details worth pinning:**

- The exact order is `GetTimeRemaining(this + 0x2C)`, test its return,
  then load/test `Production_Rate (+0x38)`. On advance, the still-live
  rate in EAX is copied to timer `Duration (+0x34)` (`disassemble_function
  0x004C9B20` at 0x004C9B63..0x004C9B97). Root cause of the prior
  Rate-vs-TimeLeft reversal: `STRUCT_FAMILY_CASCADE`.
- `Suspend` writes `IsManual`, then `IsSuspended`, then zeros Rate +0x38,
  resets timer StartTime/pad, and zeros timer Duration +0x34
  (`disassemble_function 0x004C9E60` at 0x004C9E6C..0x004C9E8E).
  Root cause of the prior timer-only description: `OPERATOR_OR_ORDER_DRIFT`.
- `Production_HasChanged` at `+0x28` is set each tick (false if no advance, true on advance). It is **distinct** from `IsDifferent` at `+0x5D`. `+0x28` is internal AI bookkeeping; `+0x5D` is the UI-refresh signal.
- The per-step cost formula has a divide-by-zero guard: when `0x36 - Production_Value == 0` (the *last* step), `costThisStep = Balance` directly instead of `Balance / 0`. This means the last step pays whatever is left in `Balance`, not 1/0 of it.
- On completion (`Production_Value == 0x36`), the remaining `Balance` is
  paid as a single `Spend_Money` call. Because StartProduction initializes
  Balance from the type cost and each successful step subtracts its spend,
  this settles the original type cost; +0x64 is not used in that accounting
  (`decompile_function 0x004C9C70` and `0x004C9B20`). Root cause of the
  prior `OriginalBalance` attribution: `INFERENCE_HARDENED`.
- **The completion branch sets `IsSuspended = true` BEFORE clearing `Object` to NULL.** The `Object` stays valid (still pointing at the produced TechnoClass) until `CompletedProduction()` is later called by `Place_Production`. This is how `FactoryClass::IsComplete` returns true (Object != NULL AND Production_Value == 0x36) for the entire window between AI-completion and placement.

### 3.3 CameoEntry.ProgressValue (+0x14, stop threshold >= 0x35)

Lives at `CameoEntry +0x14`, where CameoEntry is a slot of `StripClass`'s
embedded array (strip + 0x58 + slot × 0x34). Driven by `StripClass::AI`
(0x006A8B30), which polls each cameo's `FactoryPtr` and runs its OWN
per-cameo CDTimerClass to animate the progress bar.

```c
// StripClass::AI inner loop (per-cameo)
if (cameo.Status == 1) {                              // CameoEntry+0x10 = Building
    FactoryClass *fac = cameo.FactoryPtr;             // +0x0C
    if (fac && (fac.Production_Rate == 0
                || fac.IsSuspended
                || fac.OnHold /* via FUN_004C9C50 */)) {
        // Factory state change → reset cameo
        cameo.CameoRate = 0;                          // +0x28 = 0
        cameo.CameoTimer.StartTime = g_CurrentFrameCounter;  // +0x1C
        cameo.CameoTimer.Duration = 0;                // +0x24
        cameo.ProgressValue = 0;                      // +0x14
    }
    int remaining = CDTimerClass__GetTimeRemaining(); // cameo's own timer
    if (remaining == 0 && cameo.CameoRate != 0) {
        cameo.IsProgressingThisTick = 1;              // +0x18 (byte)
        cameo.ProgressValue += cameo.StepIncrement;   // +0x14 += +0x2C
        cameo.CameoTimer.StartTime = g_CurrentFrameCounter;
        cameo.CameoTimer.Duration = cameo.CameoRate;
        if (cameo.ProgressValue >= 0x35) {            // stop; value is not clamped
            cameo.CameoTimer.StartTime = g_CurrentFrameCounter;
            cameo.CameoRate = 0;
            cameo.CameoTimer.Duration = 0;
        }
    } else {
        cameo.IsProgressingThisTick = 0;              // +0x18
    }
}
```

**Tiny details:**

- After increment, AI compares `ProgressValue` with 0x35 and stops the
  local rate when `ProgressValue >= 0x35`; it does not clamp or rewrite
  the value. With a step greater than one it can overshoot 0x35
  (`disassemble_function 0x006A8B30` at 0x006A8FD8..0x006A9017).
  Root cause: `OPERATOR_OR_ORDER_DRIFT`.
- Draw does not pass `ProgressValue + 1` unconditionally. It starts from
  `FactoryClass::GetProgress`; only when CameoEntry.ProgressValue is
  larger does it replace the display value with the integer midpoint,
  then passes `displayValue + 1` to the progress SHP. Therefore the old
  53-frame asset conclusion does not follow from the cameo stop test
  (`decompile_function 0x006A9540`, `LAB_006a98c4` and the
  `CC_Draw_Shape(..., iStack_454 + 1, ...)` call). Root cause:
  `INFERENCE_HARDENED`.
- The OnHold check at `FUN_004C9C50` reads `factory->OnHold (+0x5C)` as a
  byte. The cameo treats `OnHold == true` the same as `IsSuspended == true`
  — both stop the cameo timer and reset progress display.
- `CameoEntry.CameoRate (+0x28)` is initialized to zero by
  `StripClass::InsertEntry`. A freshly inserted cameo does not advance
  its auxiliary progress until another path writes a non-zero rate.
  Tracking that writer
  is out of scope for this canonicalisation; an open question is logged
  in §6 (`disassemble_function 0x006A8710` at 0x006A87C3..0x006A87D7).
  Root cause of the prior field name: `STRUCT_FAMILY_CASCADE`.
- `CameoEntry.StepIncrement (+0x2C)` is also NOT touched by InsertEntry.
  Same status — set elsewhere when production actually starts.
- The "OnHold" overlay drawn by `StripClass::Draw` is gated on
  `cameo.Status == 2` OR `(FactoryPtr != NULL AND (Production_Rate == 0
  OR IsSuspended))`. So the OnHold visual fires for both "Status=2 OnHold" AND for
  a zero-rate/paused factory (`decompile_function 0x006A9540`). Root
  cause of the old +0x34 field attribution: `STRUCT_FAMILY_CASCADE`.

### 3.4 The relationship — why two fields exist

`Production_Value` is **sim state** — it must be deterministic, locked
to the network tick, and survive save/load. It's a literal "which of
the 54 cost steps have I paid for" counter and is part of the
multiplayer lockstep contract.

`CameoEntry.ProgressValue` is **UI state** — it can be reset on factory
state changes without rolling back the sim and advances on its own local
timer/rate. It is an auxiliary upper smoothing value, not the sole draw
source: Draw remains anchored to factory progress and averages toward
the cameo value only when the cameo is ahead (`decompile_function
0x006A8B30` and `0x006A9540`). Root cause of the old independent-bar
model: `INFERENCE_HARDENED`.

The flow when production advances:

1. Tick N: `FactoryClass::AI` advances `Production_Value` by 1 (one of
   54 cost steps), pays per-step cost, sets `IsDifferent = true`.
2. Same tick: `StripClass::AI` polls Rate +0x38, suspension, and OnHold;
   a stop condition zeros CameoRate and ProgressValue. Otherwise an
   expired local timer adds StepIncrement and reloads timer Duration
   from CameoRate.
3. `StripClass::Draw` reads `FactoryClass::GetProgress` first. If the
   cameo value is larger it draws `(factory + cameo) / 2 + 1`; otherwise
   it draws `factory + 1` (`decompile_function 0x006A9540`). Root cause
   of the prior source ordering: `OPERATOR_OR_ORDER_DRIFT`.

The factory and cameo timers can diverge, but the displayed frame is
still factory-anchored rather than an independent cameo-only animation.

---

## 4. CameoEntry — Canonical Field Layout (0x34 bytes total)

Embedded in `StripClass` as `Cameos[75]` starting at `StripClass + 0x58`
with stride `0x34`. Verified via `decompile_function 0x006A8710`
(StripClass::InsertEntry), `0x006A8B30` (StripClass::AI), `0x006A9540`
(StripClass::Draw), all on 2026-05-20.

| Offset | Size | Field | InsertEntry init | Notes |
|--------|------|-------|------------------|-------|
| +0x00 | 4 | TypeIndex | param_3 (TechnoTypeClass heap index) | int. Index into the array determined by `RTTIType`. |
| +0x04 | 4 | RTTIType | param_2 (WhatAmI) | int. Original RTTI code (e.g., 6 = AircraftType, 0x1F = SuperWeaponType). Used by `StripClass::Draw` to dispatch into the right TypeClass array. |
| +0x08 | 4 | NavalCheck | `RTTI_Naval_Check()` IFF `param_2 == 7` else uninitialised by InsertEntry | int. Stored ONLY when RTTIType == 7. Holds the `SpeedType` (5 = naval) from the TechnoTypeClass at +0xE08. Old docs called this `AltTypeIndex`; the actual semantics are "stored naval-classifier result for the aircraft tab selector". |
| +0x0C | 4 | FactoryPtr | 0 | `FactoryClass *`. Set externally when production starts for this cameo. NULL when nothing is being built. |
| +0x10 | 4 | Status | 0 | int. `0` = Empty, `1` = Building, `2` = OnHold, `3` = Ready. Drives draw and click-handler logic. |
| **+0x14** | **4** | **ProgressValue** | 0 | int. Auxiliary sidebar-progress value. After increment, AI stops the local rate when the value is >=0x35 but does not clamp it; Draw starts from factory progress and averages toward this value only when this value is larger (`disassemble_function 0x006A8B30`; `decompile_function 0x006A9540`). Root causes: `OPERATOR_OR_ORDER_DRIFT`, `INFERENCE_HARDENED`. |
| +0x18 | 1 | IsProgressingThisTick | not initialised by InsertEntry (defaults to whatever previous slot had after shift) | byte. AI writes 1 on the tick where ProgressValue advances and 0 otherwise (`disassemble_function 0x006A8B30` at 0x006A8FC5..0x006A9063). A Draw consumer was not established in this audit; intended downstream semantics remain UNVERIFIED. Root cause of the old contradictory "read but unused" wording: `INFERENCE_HARDENED`. |
| +0x19..+0x1B | 3 | (padding) | — | |
| **+0x1C** | **4** | **CameoTimer.StartTime** | `g_CurrentFrameCounter` | int. Frame at which the cameo's local CDTimer started. |
| +0x20 | 4 | CameoTimer.pad | `local_8` (uninitialised stack slot — Ghidra reports this as reading an uninitialised local) | int. Reserved/unused field within the embedded CDTimerClass. **Constructor reads an uninitialised stack value** (same pattern as FactoryClass +0x30). Treat as scratch. |
| +0x24 | 4 | **CameoTimer.Duration** | 0 | int. Third dword of the embedded `CDTimerClass`; remaining time is computed from +0x1C and +0x24 (`decompile_function 0x00426630`; `disassemble_function 0x006A8B30`). Root cause: `STRUCT_FAMILY_CASCADE`. |
| +0x28 | 4 | **CameoRate** | 0 | int. Separate local rate. AI tests +0x28 and copies it into timer Duration +0x24 after a step; InsertEntry initializes it to zero (`disassemble_function 0x006A8B30` at 0x006A8FCD..0x006A8FFD; `0x006A8710` at 0x006A87C3..0x006A87D7). Root cause: `STRUCT_FAMILY_CASCADE`. |
| +0x2C | 4 | **StepIncrement** | NOT touched by InsertEntry (inherits prior slot's value after shift) | int. Added to `ProgressValue` each step. Previous doc labelled this "(reserved)" — wrong; it's load-bearing. |
| **+0x30** | **4** | **FlashEndFrame** | 0 | int. Frame at which the "new item flash" effect ends. Draw compares `g_CurrentFrameCounter < this` to decide whether to render the flash overlay. The overlay's dark phase is gated on `g_CurrentFrameCounter & 0x0F > 8` (a 16-frame alternating cycle). Previous doc labelled this "FlashTimer" — the previous +0x28 and +0x30 labels were swapped. |

**Total size:** 0x34 = 52 bytes ✓ (confirmed by stride in
`StripClass::InsertEntry`'s shift loop: 13 ints × 4 bytes = 52 bytes,
and 75 × 0x34 = 0xF3C, plus the 0x58-byte StripClass header = 0xF94,
matching the documented strip size).

### 4.1 StripClass header — fields that surround the cameo array

For completeness, the StripClass fields used in conjunction with the
CameoEntry array, drawn from the patched SIDEBAR_STRIPS_TABS_CAMEOS doc
and re-verified:

| Offset | Field | Notes |
|---|---|---|
| +0x00 | AnimState[7] | Button anim ints (TS legacy?) |
| +0x1C | IsActive (byte) | Strip enabled |
| +0x3D | AutoBuild (byte) | |
| +0x3E | ScrollDirection (byte) | 0=up, 1=down (verified in prior /audit) |
| +0x3F | IsScrolling (byte) | |
| +0x44 | ScrollPosition (int) | Top row index |
| +0x48 | ScrollRequest (int) | Pending delta |
| +0x4C | ScrollPixelOffset (int) | |
| +0x50 | PrevScrollPixelOffset (int) | |
| +0x54 | CameoCount (int) | Number of live entries in Cameos[] |
| **+0x58** | **Cameos[75]** | **Array start. Stride 0x34. Each entry is the CameoEntry above.** |

---

## 5. Integration Points

- **Tick driver:** `LogicClass::AI` (0x0055AFB0) walks `g_FactoryClass_Array`
  and calls `FactoryClass::AI` (vtable[23]) for each factory each tick.
  Per the sim tick order in CLAUDE.md, this runs in the
  "scatter + production + repairs + docks + ore growth" phase.
- **Sidebar driver:** Inside `HouseClass::Update`, the
  `field_0x1FC` ProductionDirty flag triggers
  `AI_ManageProduction` + `AI_ResumeProduction`. The sidebar's
  `StripClass::AI` then polls each cameo's `FactoryPtr` to drive the
  visual progress and detect completion.
- **Save/load:** the primary vtable at `0x007E88D0` has Load/Save at
  +0x14/+0x18 = `0x004CA270`/`0x004CA3C0` (`read_memory 0x007E88D0`).
  `AbstractClass::Save` writes the full virtual size 0x74 raw, so Rate
  +0x38 and all other in-object state are included; FactoryClass then
  writes queue count/items separately. Load reconstructs vtables/vector
  storage and remaps queue items, Owner +0x6C, and Object +0x58
  (`disassemble_function 0x00410320`, `0x004CA3C0`, `0x004CA270`). Root
  causes: `GHIDRA_ADDRESS_SHIFT`, `INFERENCE_HARDENED`.
- **Network commands:** Command dispatcher at 0x004C6CB0 routes
  `0x0E → Begin_Production`, `0x0F → Suspend (FUN_004FA910)`,
  `0x10 → Cancel one (FUN_004FAA10 removeAll=0)`,
  `0x2E → Cancel all of type (FUN_004FAA10 removeAll=1)`,
  `0x0B → Place_Production` (completion).
- **Sidebar click handler:** `SelectClass::Action` at **0x006AAD00**
  (entry; the mid-body offset 0x006AB080 used to be cited in BUILD_QUEUE
  but that has been patched).

---

## 6. Open Questions — Final State of the Investigation Log

- [RESOLVED] OQ-1 — Production_Value's exact offset → +0x24, int.
  (evidence: `read_memory 0x004CA120 → 8b 41 24 c3`, 2026-05-20)
- [RESOLVED, corrected 2026-07-11] OQ-2 — `Production_Rate` is the
  separate dword at +0x38. The embedded timer occupies +0x2C..+0x34;
  +0x34 is its Duration. AI tests +0x38 after calling the timer helper
  and copies +0x38 to +0x34 on step advance (`disassemble_function
  0x004C9B20`, `0x004C9E60`, `0x004C9EA0`, `0x004CA6E0`). Root cause:
  `STRUCT_FAMILY_CASCADE`.
- [RESOLVED] OQ-3 — CameoEntry array base offset → strip + 0x58, stride 0x34.
  (evidence: `decompile_function 0x006A8710` shift loop and field writes)
- [RESOLVED, corrected 2026-07-11] OQ-4 — +0x28 is `CameoRate`,
  separate from timer Duration +0x24; +0x30 is `FlashEndFrame`
  (`disassemble_function 0x006A8B30`, `0x006A8710`; `decompile_function
  0x006A9540`). Root cause: `STRUCT_FAMILY_CASCADE`.
- [RESOLVED, corrected 2026-07-11] OQ-5 — no 53-frame asset count can
  be concluded from CameoEntry. AI stops its rate after an increment
  produces a value >=0x35 without clamping, while Draw uses factory
  progress or the midpoint toward a larger cameo value and passes that
  result +1 (`disassemble_function 0x006A8B30` at
  0x006A9000..0x006A9017; `decompile_function 0x006A9540`). Root causes:
  `OPERATOR_OR_ORDER_DRIFT`, `INFERENCE_HARDENED`.
- [RESOLVED] OQ-6 — FactoryClass.Production_Value cap → 0x36 (54) inclusive;
  `FactoryClass::IsComplete` checks `== 0x36`. (evidence:
  `decompile_function 0x004CA130`)
- [RESOLVED] OQ-7 — Default value of IsManual (+0x71) → 1 (true) on a
  fresh factory. (evidence: `decompile_function 0x004C98B0` line
  `*(undefined1 *)((int)param_1 + 0x71) = 1;`)
- [RESOLVED] OQ-8 — Is `FUN_004C9C50` related to the cameo state-change
  check? → Yes; it's a getter for `FactoryClass.OnHold (+0x5C)`. The
  cameo treats `OnHold == true` the same as `IsSuspended == true` for
  reset purposes.
  (evidence: `decompile_function 0x004C9C50` returns `*(byte *)(param_1 + 0x5C)`)
- [RESOLVED] OQ-9 — Production_HasChanged (+0x28) vs IsDifferent (+0x5D)
  → Two separate bytes with two separate roles. +0x28 is per-tick AI
  bookkeeping (set true exactly on the tick where Production_Value
  advances); +0x5D is the UI-refresh signal. (evidence:
  `decompile_function 0x004C9B20`)
- [RESOLVED] OQ-10 — Is `Production_Step (+0x3C)` ever non-1 in YR?
  → No code path writes anything other than 1 to this field. Constructor
  initialises to 1; no setter exists. Effectively a constant.
  (evidence: `disassemble_function 0x004C98B0` and whole-program
  `search_instructions(mnemonic=MOV, operand_pattern=0x3c])`, whose only
  FactoryClass matches are the constructor write and AI read.)
- [RESOLVED] OQ-11 — CameoEntry +0x18 byte semantics → `IsProgressingThisTick`,
  set 1 on the tick where ProgressValue advanced; 0 otherwise. Currently
  not read by Draw (possibly TS legacy or unused signal).
  (evidence: `decompile_function 0x006A8B30`)
- [RESOLVED] OQ-12 — CameoEntry +0x08 (`NavalCheck`) initialisation →
  Set by InsertEntry only when RTTIType == 7 (Aircraft); for other
  RTTI types it is left at whatever the shift loop copied from the
  prior slot. This is mildly hazardous but is gated upstream by the
  RTTIType check at every read site. (evidence:
  `decompile_function 0x006A8710`)
- [DEFERRED] OQ-13 — Where is `CameoEntry.CameoRate (+0x28)`
  set to a non-zero value? `InsertEntry` initializes it to 0 but the AI loop
  doesn't animate progress until it's non-zero. The writer is not established
  here. (category: `requires-different-system-context`;
  reason: belongs in a production-start-pipeline trace, not a struct-layout
  doc; next-step-if-pursued: trace `StripClass::AddCameo` and any setter
  in `SidebarClass::AddCameo`/`Begin_Production` that writes to the
  cameo's +0x28.) (`disassemble_function 0x006A8710` and `0x006A8B30`;
  `INFERENCE_HARDENED` removed.)
- [DEFERRED] OQ-14 — Where is `CameoEntry.StepIncrement (+0x2C)` set?
  Same pattern as OQ-13. (category: `requires-different-system-context`;
  next-step-if-pursued: same as OQ-13.)
- [DEFERRED] OQ-15 — Why does the constructor write `local_8` (an
  uninitialised stack slot) into `+0x30` (Timer.pad)? Compiler artefact
  or intentional? Same pattern in CameoEntry InsertEntry writing
  `local_8` into +0x20. (category: `bounded-cost-too-high`; reason:
  needs assembly-level analysis of the prologue and the compiler used;
  next-step-if-pursued: dump the constructor's first ~30 bytes and check
  whether `local_8` corresponds to a register the calling convention
  reserves.)

## Sources

**Ghidra functions re-verified in this correction session (2026-07-11):**

- 0x004C98B0 — FactoryClass constructor entry
- 0x004C9B20 — FactoryClass::AI
- 0x004C9E60 — FactoryClass::Suspend
- 0x004C9EA0 — FactoryClass::SetRate/Resume
- 0x004C9FB0 — FactoryClass::CalcRate
- 0x004C9C50 — FactoryClass::GetOnHold (returns +0x5C byte)
- 0x004C9FF0 — FactoryClass::AbandonProduction entry
- 0x004CA270 — FactoryClass::Load
- 0x004CA3C0 — FactoryClass::Save
- 0x004CA120 — FactoryClass::GetProgress (also raw-bytes read)
- 0x004CA130 — FactoryClass::IsComplete (also raw-bytes read)
- 0x004CA430 — FactoryClass debug dump (vtable[13])
- 0x004CA6E0 — FactoryClass::RecalcAllRates
- 0x00426630 — CDTimerClass::GetTimeRemaining
- 0x006A8710 — StripClass::InsertEntry
- 0x006A8B30 — StripClass::AI
- 0x006A9540 — StripClass::Draw

**Prior research consulted:**

- `FACTORYCLASS_PRODUCTION_DEEP_DIVE.md` — high-confidence baseline for FactoryClass methods and lifecycle
- `FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md` — high-confidence baseline for SetRate / GetBuildStepTime
- `BUILD_QUEUE_GHIDRA_REPORT.md` — its corrected +0x38 Rate/+0x34 timer split agrees with the live instructions re-verified here.
- `SIDEBAR_STRIPS_TABS_CAMEOS_GHIDRA.md` — cited for historical context only; sibling claims were not edited in this slot.
- `SIDEBAR_SYSTEM_GHIDRA_REPORT.md` — its 2026-07-10 correction now agrees that CameoEntry +0x24 is timer Duration and +0x28 is Rate; sibling claims were not edited here.

**INI keys cross-checked:**

- `RulesClass.MaximumQueuedObjects` at RulesClass +0xF0 (from FactoryClass::StartProduction)
- `RulesClass.MultipleFactory` at RulesClass +0x57C (from GetBuildStepTime — out of scope for this struct doc)
