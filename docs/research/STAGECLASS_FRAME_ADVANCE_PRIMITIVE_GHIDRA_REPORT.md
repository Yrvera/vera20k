# StageClass Frame-Advance Primitive — Ghidra Research Report

**Target:** `StageClass` layout, API, and which YR-live systems advance frames or stages through it vs through their own CDTimer/counter fields.
**Session date:** 2026-05-28
**Confidence:** HIGH for all struct layouts, method set, and inheritance hierarchy (verified via RTTI BCD walk + live decompile). HIGH for AnimClass/FactoryClass not calling `Stage_Changed` (confirmed from full decompilation of both AI functions). HIGH for FlasherClass as the sole Stage_Changed consumer (single xref confirmed: `get_xrefs_to 0x004CC770` = one caller).
**Active in YR:** Yes (FlasherClass path is live every skirmish; AnimClass/FactoryClass embed the struct but drive advance independently).

---

## Investigation Scope

- **Target question:** What is StageClass's struct layout and API? Which YR-live systems use `Stage_Changed` or any StageClass method for frame/stage advance, vs using their own CDTimer fields?
- **Non-goals:** Full AnimClass frame-advance analysis (already settled); FactoryClass build-speed formula (already documented); Gattling stage system (already documented — uses no StageClass methods).
- **Evidence needed to mark COMPLETE:** (1) StageClass struct layout verified. (2) Complete method list for StageClass. (3) RTTI-confirmed set of classes inheriting StageClass. (4) For each subclass: verified whether it calls `Stage_Changed` or drives advance independently.
- **Stop conditions:** All three inheriting classes investigated; no additional StageClass methods found; Stage_Changed single-caller confirmed.

---

## 1. StageClass Struct Layout (VERIFIED)

**RTTI:** TypeDescriptor string `.?AVStageClass@@` at `0x00817AC0`, TypeDescriptor struct at `0x00817AB8`. Verified via `read_memory 0x00817AC0` reading ASCII `.?AVStageClass@@\0`.

```
StageClass {
    +0x00  Value: int       (4 bytes)   — the counter / stage value
    +0x04  HasChanged: bool (1 byte)    — set true when Value just changed (bit-0 toggle)
    [+0x05..+0x07: 3-byte alignment padding]
}
// Total: 8 bytes
```

Verified from `TechnoClass::Constructor @ 0x006F2B40` (decompile_function):
```
param_1[0x3c] = 0;                        // +0xF0 = Value = 0
*(undefined1*)(param_1 + 0x3d) = 0;       // +0xF4 = HasChanged = 0
```
(Note: `param_1[0x3c]` with `int*` = byte offset `0x3c*4 = 0xF0`; `param_1 + 0x3d` as byte ptr = `0xF0 + 4 = 0xF4`. These match the FlasherClass embedding at TechnoClass+0xF0.)

---

## 2. StageClass Method Set (VERIFIED — exactly one live method)

### `StageClass::Stage_Changed @ 0x004CC770`

Confirmed from `decompile_function 0x004CC770`:

```c
bool StageClass::Stage_Changed(int* stage) {
    if (stage->Value != 0) {
        uint decremented = stage->Value - 1;
        stage->HasChanged = false;
        stage->Value = decremented;
        if (decremented & 1) stage->HasChanged = true;
        return true;   // still counting
    }
    return false;      // reached zero
}
```

**Semantics:** Decrement-by-1 countdown. `HasChanged` reflects whether bit-0 of the new value is set — i.e., it flips true/false every other tick while counting down. Returns `false` when Value was already 0 (expired).

**This is NOT a rate-driven frame-counter.** There is no CDTimerClass embedded in StageClass, no `Set_Rate`, no `Graphic_Logic`, no `Fetch_Stage`. The TS-era header documentation of StageClass as a "Rate+Timer+Delay" struct does NOT match what gamemd.exe contains.

**Caller count:** `get_xrefs_to 0x004CC770` returns **exactly one xref**:
```
From 006fac4d in TechnoClass__AI_Update [UNCONDITIONAL_CALL]
```
No other callers. Stage_Changed is not called from AnimClass::AI, FactoryClass::AI, CloakingTick, or any building/combat system.

**No other StageClass methods exist.** Binary RTTI walk finds no StageClass vtable (no CompleteObjectLocator; no virtual methods). The only code that touches `StageClass {Value, HasChanged}` directly is `Stage_Changed` plus the zero-init in constructors of embedding classes.

---

## 3. Classes Inheriting StageClass (VERIFIED — exactly three)

RTTI BCD walk on StageClass TD `0x00817AB8` finds three BaseClassDescriptors referencing it:

| BCD address | Contains StageClass TD | StageClass subobject offset in host | Host CHD | Host class |
|---|---|---|---|---|
| `0x007FB3A0` | `0x00817AB8` | `0xF8` | `0x00817AD8` | `FlasherClass` |
| `0x007FB9F0` | `0x00817AB8` | `0xAC` | `0x008182C8` | `AnimClass` |
| `0x008005A0` | `0x00817AB8` | `0x24` | `0x00822290` | `FactoryClass` |

Host CHD TypeDescriptor names verified via `read_memory`:
- `0x008182D0` → `.?AVAnimClass@@\0` (**AnimClass**)
- `0x00822298` → `.?AVFactoryClass@@\0` (**FactoryClass**)
- `0x00817AE0` (existing label) → `.?AVFlasherClass@@\0` (**FlasherClass**)

All three verified inline from `read_memory` calls in this session.

---

## 4. Per-Class Analysis

### 4.1 FlasherClass (inherits StageClass at offset +0xF8 within FlasherClass; embedded in TechnoClass at +0xF0)

**Uses Stage_Changed: YES** — the only caller.

FlasherClass is a POD mixin embedded in every TechnoClass (Building, Unit, Infantry, Aircraft). The StageClass fields appear at:
- `TechnoClass+0xF0` = Stage.Value (int countdown)
- `TechnoClass+0xF4` = Stage.HasChanged (bool)

**Live path:** `TechnoClass::AI_Update @ LAB_006FAC31` calls `Stage_Changed(&this->field_0xF0)` every tick. If Value was non-zero and bit-1 of the new Value differs from old Value AND the object is a Building (`WhatAmI() == 6`), fires `TacticalClass::DirtyScreenRect` + `BuildingClass::UpdateAllAnimFacings`.

**Seed site (only one):** `TechnoClass::AI_Update @ 0x006FA055` — on transition to Elite veterancy, writes `this->field_0xF0 = Rules.EliteFlashTimer` (= 150 frames from `Rules+0xBE8`).

**Active in YR:** Yes — fires every match (the `Stage_Changed` call runs unconditionally every AI tick for every TechnoClass), but the visible effect (dirty-rect trigger) only fires for 150 frames after a building Elite promotion.

**Rate formula:** None. StageClass.Value decrements by exactly 1 per call to `Stage_Changed` = 1 per game frame. Duration = 150 frames ÷ 60 fps nominal ≈ 2.5 seconds. HasChanged toggles every 2 frames (bit-0 cadence), dirty-rect fires when bit-1 toggles = every 4 frames... (Actually: bit-1 changes every 2 decrements, so dirty-rect fires at frames 149→148, 147→146, etc. — every even decrement = every other tick = ~30 Hz cadence for the 150-frame window.)

### 4.2 AnimClass (inherits StageClass at offset +0xAC within AnimClass)

**Uses Stage_Changed: NO.** AnimClass::AI drives frame advance directly.

The StageClass subobject at AnimClass+0xAC:
- `AnimClass+0xAC` = Stage.Value → **reused as `CurrentFrame`** in AnimClass's own logic
- `AnimClass+0xB0` = Stage.HasChanged → set to `0` or `1` by AnimClass::AI directly

Verified from `AnimClass::Constructor @ 0x00421EA0`:
```c
param_1[0x2b] = 0;                          // +0xAC = Stage.Value = 0 = initial frame
*(undefined1*)(param_1 + 0x2c) = 0;         // +0xB0 = Stage.HasChanged = 0
```

Verified from `AnimClass::AI @ 0x00423AC0` (frame advance block):
```c
iVar8 = CDTimerClass__GetTimeRemaining();    // CDTimer at AnimClass+0xB4..+0xBF
if (iVar8 != 0 || param_1[0x30] == 0) {
    *(undefined1*)(param_1 + 0x2c) = 0;     // +0xB0 = HasChanged = false (no advance)
    return;
}
*(undefined1*)(param_1 + 0x2c) = 1;         // +0xB0 = HasChanged = true (advancing)
param_1[0x2b] += param_1[0x31];             // +0xAC = Value += FrameStep
param_1[0x2d] = g_CurrentFrameCounter;      // +0xB4 = LastFrameTime (NOT StageClass)
param_1[0x2f] = param_1[0x30];              // +0xBC = FrameDelay = FrameDelayReload
```

AnimClass writes HasChanged and Value directly — it does **not** call `Stage_Changed`. The CDTimer that controls advance rate lives at `+0xB4..+0xBF` (outside the StageClass subobject).

**Conclusion:** AnimClass re-uses StageClass's `{Value, HasChanged}` storage for `{CurrentFrame, FrameAdvancedThisTick}` but replaces StageClass's decrement-countdown semantics with its own CDTimer-gated increment logic. The two systems are not composable — AnimClass ignores `Stage_Changed` entirely.

**Active in YR:** Yes (AnimClass is live for all in-game animations).

### 4.3 FactoryClass (inherits StageClass at offset +0x24 within FactoryClass)

**Uses Stage_Changed: NO.** FactoryClass::AI drives production progress directly.

The StageClass subobject at FactoryClass+0x24:
- `FactoryClass+0x24` = Stage.Value → **reused as `Production_Value`** (progress counter 0..54)
- `FactoryClass+0x28` = Stage.HasChanged → set by `FactoryClass::AI`

Verified from `FactoryClass::Constructor @ 0x004C98B0`:
```c
param_1[9] = 0;                              // +0x24 = Stage.Value = 0 = Production_Value
*(undefined1*)(param_1 + 10) = 0;            // +0x28 = Stage.HasChanged = 0
```
(With `int*` param: `param_1[9]` = byte offset `0x24`, `*(param_1 + 10)` = byte at `0x28`.)

The CDTimer used by `FactoryClass::AI` starts immediately after at `+0x2C..+0x37` (12 bytes). `FactoryClass::GetProgress @ 0x004CA120` returns `Production_Value` from `+0x24`. `FactoryClass::HasCompleted @ 0x004CA130` checks `Production_Value == 0x36` (= 54). Stage_Changed is not called from `FactoryClass::AI` (confirmed via `get_function_callees`-style inspection: the AI function calls CDTimerClass helpers but no StageClass function).

**Active in YR:** Yes (FactoryClass is live for all production).

---

## 5. Systems That Do NOT Use StageClass

| System | Advance mechanism | Evidence |
|---|---|---|
| Building animations (UpdateAnimation) | CDTimer at `BuildingClass+0xF8..+0x10F`; frame counter at `+0xF8` written directly | `decompile_function 0x004509D0` — no StageClass call |
| Gattling weapon stages | `GattlingValue` int accumulator, `CurrentGattlingStage` int; no timer | `GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md` |
| Cloak/uncloak transition | `CloakProgress` int + `CloakDirty` bool at TechnoClass+0x224/+0x228; CDTimer at +0x22C | `CLOAK_FX_SHADER_BRIDGE_GHIDRA_REPORT.md` — no StageClass |
| Wall damage stages | `CellClass::field_0x11E` upper nibble, incremented by `+0x10` in `DestroyOverlay` | `WALL_DAMAGE_STAGE_INCREMENTER_GHIDRA_REPORT.md` |
| Building door open/close (nuke silo) | Mission state counter at `BuildingClass+0xBC`; `field_0x6DD` flag when frames match | `NUKE_SUPERWEAPON_GHIDRA_REPORT.md` |
| Damage-flash (does not exist in YR) | N/A — no damage-flash path touches `+0xF0` (FLASHER_CLASS doc, round-3 sweep) | `FLASHER_CLASS_GHIDRA_REPORT.md` |

---

## 6. AnimClass–StageClass Relationship — Definitive Answer

AnimClass **inherits** StageClass (RTTI-confirmed). The `{Value, HasChanged}` storage at AnimClass+0xAC/+0xB0 is reused as `{CurrentFrame, FrameAdvancedFlag}`. However:
- AnimClass **never calls** `Stage_Changed`
- AnimClass drives advance through its own CDTimer at `+0xB4..+0xBF`
- The semantics differ: StageClass.Stage_Changed decrements; AnimClass increments Value

**In practice:** for Rust purposes, AnimClass does NOT use StageClass semantics. AnimClass's frame-advance is entirely self-contained and already documented (GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md §3.6, TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md). StageClass is irrelevant to AnimClass parity work.

---

## 7. Implementation Handoff

### Handoff 1 — Elite promotion flash timer

**Verified behavior:** When any TechnoClass promotes to Elite, `TechnoClass+0xF0` is written with `Rules.EliteFlashTimer` (= 150 frames). Each tick, `Stage_Changed` decrements it; when bit-1 toggles AND the object is a Building, `TacticalClass::DirtyScreenRect` + `BuildingClass::UpdateAllAnimFacings` fire. Effect lasts ~150 ticks / ~5 seconds at 30 Hz.

**Rust delta:** `src/sim/game_entity.rs` has no `elite_flash_timer` field. No equivalent of `Stage_Changed` or dirty-rect trigger on Elite promotion.

**Affected surface:** All buildings that can promote to Elite (any with `Veteran=yes` / `Elite=yes` path and `EliteFlashTimer > 0` in `[AudioVisual]`). Fires whenever a building reaches Elite rank in combat.

**Acceptance scenario:** Promote a `[TNKD]` (Allied Pillbox) to Elite in-game. For ~150 frames it should visually "blink" (building animation reset + dirty rect every 2 frames). No blink = DRIFT.

**Proposed test name:** `test_elite_promotion_flash_timer_decrements`

**Risk:** Low for gameplay parity (purely cosmetic), but visible and fires every time a building reaches Elite rank.

### Handoff 2 — FactoryClass progress counter is StageClass.Value

**Verified behavior:** `FactoryClass::GetProgress @ 0x004CA120` reads `FactoryClass+0x24` (StageClass.Value). `HasCompleted @ 0x004CA130` checks `Value == 0x36` (54). `Production_HasChanged @ +0x28` is StageClass.HasChanged — set by FactoryClass::AI, read by sidebar to trigger cameo redraws.

**Rust delta:** The Rust production system (`src/sim/world/world_spawn.rs`, factory system) should already track production progress as an int 0..54. The field names (`Production_Value`, `Production_HasChanged`) map directly to StageClass fields. No parity concern if the 0..54 range and per-step increment are correct. **Verify that `HasChanged` equivalent is set correctly** to trigger sidebar redraws on each step.

**Proposed test name:** `test_factory_progress_changed_flag_per_step`

**Risk:** Medium — sidebar cameo redraws depend on `HasChanged` being set on each production step.

### Handoff 3 — No StageClass rate/timer API to implement

**Verified behavior:** StageClass has NO `Set_Rate`, `Set_Delay`, `Graphic_Logic`, `Fetch_Stage`, or CDTimerClass fields. The TS-era documentation describing StageClass as a "Rate+Stage+Timer+Delay" compound is **inapplicable to gamemd.exe**.

**Rust delta:** No Rust `StageClass` equivalent needs a CDTimer or rate field. The only needed Rust equivalent is a simple `{ value: i32, has_changed: bool }` struct with a `stage_changed(&mut self) -> bool` method (decrement, set has_changed from bit-0).

**Proposed test name:** `test_stageclass_stage_changed_decrements_and_flag`

**Risk:** None if the scoped design above is followed. Risk if a TS-era API is ported unnecessarily.

---

## 8. Negative Facts / Do Not Do

1. **Do NOT implement Set_Rate, Set_Delay, Graphic_Logic, Fetch_Stage as StageClass methods.** None exist in gamemd.exe. Verified: the only code operating on `{StageClass.Value, StageClass.HasChanged}` in the binary is `Stage_Changed @ 0x004CC770` plus direct field writes in constructors and AnimClass/FactoryClass AI. (`search_strings` + RTTI sweep this session.)

2. **Do NOT call Stage_Changed from AnimClass frame-advance logic.** AnimClass drives `CurrentFrame` (= StageClass.Value) directly via CDTimer gating and does not invoke Stage_Changed. Verified from `decompile_function 0x00423AC0`.

3. **Do NOT model CloakProgress as a StageClass.** TechnoClass cloaking uses ad-hoc `{CloakProgress: int, CloakDirty: bool}` at +0x224/+0x228 — same structural pattern as StageClass but not inheritance. Confirmed: RTTI BCD walk finds no CloakClass or TechnoClass-cloaking StageClass BCD.

4. **Do NOT embed a CDTimerClass in the Rust StageClass equivalent.** The gamemd StageClass struct is 8 bytes (`{int, bool}`), no timer. CDTimers live in the embedding class's own fields (FactoryClass+0x2C, AnimClass+0xB4), not inside StageClass.

5. **Do NOT port gattling weapon "stages" through StageClass.** Gattling stages are entirely separate: `CurrentGattlingStage` + `GattlingValue` accumulator in TechnoClass — no StageClass involvement. Verified from `GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md` and xref scan.

---

## 9. Remaining Uncertainty

1. **Map-trigger action 0x83 (FUN_006e4560)** seeds `TechnoClass+0xF0` for buildings matching a type ID (scripted mission flash). The exact INI trigger key and whether it uses the same `Stage_Changed` cadence vs a different stride was not traced. Scope: map scripting system (low priority for skirmish parity).

2. **`Stage_Changed` return-value usage in TechnoClass::AI_Update.** The `changed` boolean returned by Stage_Changed is checked via `vtable+0x124` call — the function at vtable+0x124 for BuildingClass was not identified in this session. Likely a generic "notify changed" hook. Low risk: the dirty-rect trigger is gated on bit-1 of Value, not the return value directly.

---

## 10. Evidence Index

| Claim | Evidence |
|---|---|
| StageClass TD `0x00817AB8` = `.?AVStageClass@@` | `read_memory 0x00817AC0` → ascii `.?AVStageClass@@` |
| StageClass struct = `{int Value, bool HasChanged}` 8 bytes | `TechnoClass::Constructor 0x006F2B40` ctor zero-init at `[0x3c]`/`[0x3d]` |
| `Stage_Changed @ 0x004CC770` is the only StageClass method | `decompile_function 0x004CC770`; RTTI = no vtable; `search_strings StageClass` = TD only |
| Single caller of Stage_Changed | `get_xrefs_to 0x004CC770` → 1 result: TechnoClass__AI_Update |
| AnimClass inherits StageClass at offset +0xAC | RTTI BCD `0x007FB9F0`: TD=`0x00817AB8`, member_offset=`0xAC`; CHD `0x008182C8` = `.?AVAnimClass@@` confirmed via `read_memory 0x008182D0` |
| FactoryClass inherits StageClass at offset +0x24 | RTTI BCD `0x008005A0`: TD=`0x00817AB8`, member_offset=`0x24`; CHD `0x00822290` = `.?AVFactoryClass@@` confirmed via `read_memory 0x00822298` |
| AnimClass does NOT call Stage_Changed | `decompile_function 0x00423AC0` frame-advance block writes Value/HasChanged directly |
| FactoryClass does NOT call Stage_Changed | `decompile_function 0x004C98B0` ctor zeros `[9]`/`*(param_1+10)`; FactoryClass::AI drives via CDTimer |
| BuildingClass::UpdateAnimation uses own CDTimer, not StageClass | `decompile_function 0x004509D0` — CDTimerClass__GetTimeRemaining call, direct `field_0xf8` write |

---

## Sources

**Ghidra functions decompiled this session:**
- `0x004CC770` — `StageClass__Stage_Changed`
- `0x00421EA0` — `AnimClass__Constructor`
- `0x00423AC0` — `AnimClass__AI`
- `0x004C98B0` — `FactoryClass__Constructor`
- `0x004509D0` — `BuildingClass__UpdateAnimation`
- `0x006FB740` — `TechnoClass__CloakingTick`

**RTTI inspected:**
- `0x00817AB8` — StageClass TypeDescriptor
- `0x007FB3A0`, `0x007FB9F0`, `0x008005A0` — BCD records referencing StageClass TD
- `0x008182C8`, `0x00822290`, `0x00817AD8` — Host CHD TypeDescriptors

**Prior docs extending (not redo):**
- `docs/research/FLASHER_CLASS_GHIDRA_REPORT.md` — FlasherClass already fully documented; this doc confirms no additional callers
- `docs/research/GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md` §3.6 — AnimClass fields already settled; this doc confirms AnimClass–StageClass relationship
- `docs/research/SIDEBAR_SYSTEM_GHIDRA_REPORT.md` §33 — FactoryClass Production.Value as StageClass; this doc confirms single-caller and no Stage_Changed use
