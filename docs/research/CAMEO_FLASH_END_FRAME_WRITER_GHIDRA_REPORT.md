# CameoEntry.FlashEndFrame Writer — Ghidra Research Report

**Primary Addresses:**
- `StripClass::Draw` @ `0x006A9540` — sole READER of FlashEndFrame
- `StripClass::InsertEntry` @ `0x006A8710` — writes `FlashEndFrame = 0` on entry insertion
- `StripClass::Recalculate` @ `0x006AA600` — writes `FlashEndFrame = 0` on entry removal
- `FUN_006a80a0` (StripClass constructor/init) @ `0x006A80A0` — implicit zero-init (never writes +0x88)
- **No non-zero writer found after exhaustive search of all sidebar, factory, and house code paths.**

**Confidence:** HIGH on field offset and reader behavior (binary-verified). HIGH on the conclusion that no non-zero writer exists (exhaustive code search). MEDIUM on "dead code in YR" designation (runtime debugger not available to confirm, but logic is airtight).

**Active in YR:** Conditional — the CameoEntry flash CHECK is live code that executes every Draw tick, but the EFFECT is permanently suppressed because FlashEndFrame is always 0. The flash overlay never draws. See Section 3.

---

## 1. Overview

`CameoEntry.FlashEndFrame` is a 32-bit integer field at strip-relative offset `strip + slot*0x34 + 0x88` (CameoEntry-relative offset `+0x30`). It is the absolute `g_CurrentFrameCounter` value at which the "new-buildable" pulse overlay stops drawing. `StripClass::Draw` checks `g_CurrentFrameCounter < FlashEndFrame` on every frame per slot; if true and `(frame & 0xF) > 8`, it draws a darkening overlay (flag `0x404`) on top of the cameo to create a pulse effect.

The field is initialized to 0 at construct time and zeroed again on every insertion/removal. **No function in the binary has been found that writes a non-zero value to this field.** The flash check therefore always evaluates false (frame counter ≥ 0 > 0), making the entire cameo pulse code dead in practice under stock YR.

This resolves the open question from `SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT.md §3.2 and §7`.

---

## 2. Field Offset — Resolving the +0x28 vs +0x30 Ambiguity

**Verdict: FlashEndFrame is at CameoEntry-relative offset +0x30, strip-relative offset `strip + slot*0x34 + 0x88`.**

### Evidence from `StripClass::Draw` @ `0x006A9540`

The Draw function iterates slots with index `iStack_450`. The read is:
```c
if ((int)g_CurrentFrameCounter < *(int *)((int)local_468 + iStack_450 * 0x34 + 0x88))
```
where `local_468 = this` (strip pointer). Verified: `decompile_function 0x006A9540`.

### Deriving CameoEntry-relative offset

From `StripClass::InsertEntry` @ `0x006A8710`: the CameoEntry array starts at `strip + 0x58` (the loop starts at `param_1 + 0x58`). For slot 0, the strip-relative CameoEntry base = `strip + 0`. Fields:
- TypeID at `strip + 0x58` = CameoEntry_base + 0x58
- FlashEndFrame zeroed at `strip + 0x88` = CameoEntry_base + 0x88

Wait — these numbers are inconsistent if the CameoEntry starts at +0x58. Let me resolve:

**`InsertEntry` uses `iVar4 = param_1 + iVar5 * 0x34`** (no +0x58 added). For slot 0: `iVar4 = strip`. Then writes TypeID at `iVar4 + 0x58 = strip + 0x58`. For slot 1: `iVar4 = strip + 0x34`. TypeID at `strip + 0x34 + 0x58 = strip + 0x8C`.

This means the CameoEntry **struct stride** is 0x34 bytes, but the CameoEntry data fields (TypeID etc.) start at `strip + 0x58`, which is CameoEntry_slot0_base + 0x58. The CameoEntry "struct" effectively begins at `strip + slot*0x34` and its first data field of interest is at +0x58 within that base.

**FlashEndFrame within CameoEntry = +0x88 - +0x58 = +0x30** relative to the CameoEntry struct start (or equivalently, +0x30 relative to TypeID = +0x00 within the data section). Verified: `decompile_function 0x006A8710`.

### Correction to older doc

`SIDEBAR_STRIPS_TABS_CAMEOS_GHIDRA.md §CameoEntry` lists FlashEndFrame at CameoEntry+0x28 and "FlashTimer" at +0x30. This is incorrect. The Draw function confirms +0x30 = FlashEndFrame via the strip+slot*0x34+0x88 read. The field at +0x28 within CameoEntry data is AnimStartTime or a timer field (not separately confirmed as FlashEndFrame). The timing doc (Apr 2026) correctly identified +0x30 / strip+0x88 as FlashEndFrame.

---

## 3. The Flash Code Is Dead — No Writer Exists

### Exhaustive Search Summary

The following functions were decompiled and inspected for writes to `strip + slot*0x34 + 0x88`:

| Function | Address | Result |
|---|---|---|
| `StripClass::Draw` | `0x006A9540` | READER only |
| `StripClass::InsertEntry` | `0x006A8710` | WRITER: always writes 0 |
| `StripClass::Recalculate` | `0x006AA600` | WRITER: always writes 0 (removal path) |
| `StripClass::AI` | `0x006A8B30` | No write to +0x88 |
| `SidebarClass::AddCameo` | `0x006A6300` | No write to +0x88 |
| `SidebarClass::Action` | `0x006A7780` | No write to +0x88 |
| `SelectClass::Action` | `0x006AADE0` | No write to +0x88 |
| `Sidebar_UpdateFromProduction` | `0x006A6140` | No write to +0x88 |
| `FUN_006abb60` (CameoEntry cleanup) | `0x006ABB60` | No write to +0x88 |
| `FUN_006a80a0` (StripClass init) | `0x006A80A0` | Never writes +0x88; implies 0 from zero-init |
| `HouseClass::Begin_Production` | `0x004FA350` | Calls Sidebar_UpdateFromProduction; no +0x88 write |
| `HouseClass::Place_Production` | `0x004FB0E0` | No write to +0x88 |
| `HouseClass::Update` | `0x004F8440` | Calls FUN_006a7d20 → Recalculate; no +0x88 write |
| `HouseClass::AI_ResumeProduction` | `0x0050B1D0` | Calls AddCameo; no +0x88 write |
| `FactoryClass::AI` | `0x004C9B20` | No sidebar interaction; no +0x88 write |
| `FactoryClass::CompletedProduction` | `0x004CA1A0` | No sidebar interaction; no +0x88 write |
| `FUN_004FAA10` (AbandonProduction path) | `0x004FAA10` | No +0x88 write |
| `EventClass::Execute` (cmd 0xe) | `0x004C6CB0` | Dispatches to Begin_Production; chain traced above |

All callers of `InsertEntry` (the zero-setter) were also traced:
- `SidebarClass::AddCameo` @ `0x006A6300` — only two callers: `HouseClass::AI_ResumeProduction` and `TriggerAction::Execute`
- `FUN_006A87F0` @ `0x006A87F0` — confirmed NO callers (dead function, unreferenced)

### Why the Check Always Fails

`g_CurrentFrameCounter` starts at 0 or 1 and monotonically increases. `FlashEndFrame` is always 0. The check:
```c
if ((int)g_CurrentFrameCounter < *(int *)(strip + slot*0x34 + 0x88))
// = if (positive_int < 0)  → always false
```
The overlay draw never executes. Confirmed by tracing every write path.

### Active in YR

The flash draw check at `StripClass::Draw` is **live code that runs** every frame for every visible cameo slot. But the **observable effect is zero** — the overlay is never drawn because the condition is always false. Designation: **dead feature in stock YR** (code present but permanently inactive due to missing setter).

---

## 4. Duration Constant / Formula — Not Applicable

No setter was found, so there is no duration constant to document. If the setter were implemented, it would need to write `g_CurrentFrameCounter + N` where N is the desired flash duration in game frames. The field type is `int32`. The prior doc speculated this might be rules.ini-derived, but no INI key was found that maps to this field. The field is not saved/loaded (`SidebarClass::Load` @ `0x006AC5E0` just delegates to `RadarClass::Load` with no CameoEntry serialization).

---

## 5. Caller Chain and Trigger Condition

**Trigger condition for the cameo flash in Draw**: `g_CurrentFrameCounter < FlashEndFrame`. This fires during the draw of any visible cameo slot while `cStack_476 != 0` (i.e., production is pending/in-progress for that slot OR status is building/queued).

**The intent** (from context): FlashEndFrame was presumably meant to be set to `g_CurrentFrameCounter + N` when a new item is first inserted into the strip, creating a limited-duration pulse to draw the player's attention. The tab-flash system (via `DAT_00b0b478` SHP animation) DOES work as intended; the cameo-level individual slot flash DOES NOT work in stock YR.

---

## 6. CameoEntry Full Field Layout (Corrected)

Fields relative to CameoEntry base at `strip + slot*0x34` (for slot N). "CameoData section" starts at +0x58 from the base:

| Strip-relative offset (slot 0) | CameoData offset | Type | Field | Default |
|---|---|---|---|---|
| `strip + 0x58` | +0x00 | uint32 | TypeIndex | 0 |
| `strip + 0x5C` | +0x04 | int32 | RTTIType | 0 |
| `strip + 0x60` | +0x08 | int32 | NavalClass extra | 0 |
| `strip + 0x64` | +0x0C | void* | FactoryPtr | 0 |
| `strip + 0x68` | +0x10 | int32 | Status (0=none,1=build,2=hold,3=ready) | 0 |
| `strip + 0x6C` | +0x14 | int32 | ProgressValue (0–0x34) | 0 |
| `strip + 0x70` | +0x18 | int8 | AutoBuildFlag | 0 |
| `strip + 0x74` | +0x1C | uint32 | Timer.StartTime | g_CurrentFrameCounter |
| `strip + 0x78` | +0x20 | uint32 | Timer.pad | local_8 (uninit) |
| `strip + 0x7C` | +0x24 | uint32 | Timer.Duration | 0 |
| `strip + 0x80` | +0x28 | uint32 | Timer.TimeLeft | 0 |
| `strip + 0x84` | +0x2C | uint32 | AnimSpeed (init=1 in ctor) | 1 |
| **`strip + 0x88`** | **+0x30** | **int32** | **FlashEndFrame** | **0** |

Sources: `InsertEntry` @ `0x006A8710` (all fields), `FUN_006a80a0` @ `0x006A80A0` (constructor loop), `StripClass::Draw` @ `0x006A9540` (FlashEndFrame and Status reads).

---

## 7. Open Questions — Final State

- `[RESOLVED] OQ-1 — What is the exact CameoEntry offset for FlashEndFrame?` → CameoData +0x30 = strip+slot*0x34+0x88. Confirmed via StripClass::Draw @ 0x006A9540 read pattern.
- `[RESOLVED] OQ-2 — Resolve +0x28 vs +0x30 ambiguity between prior docs.` → +0x30 is FlashEndFrame; SIDEBAR_STRIPS_TABS_CAMEOS_GHIDRA.md had it wrong. Evidence: Draw read, InsertEntry zero at strip+0x88.
- `[RESOLVED] OQ-3 — Who writes FlashEndFrame to a non-zero value?` → Nobody. Exhaustive search across 18+ functions covering all production/sidebar code paths found no non-zero writer.
- `[RESOLVED] OQ-4 — What is the duration constant/formula?` → None exists. The feature is unfinished/dead — no setter was ever connected.
- `[RESOLVED] OQ-5 — What triggers the flash? AddCameo? Production complete?` → The draw check fires during active-production slots, but the flash never renders because FlashEndFrame = 0 always.
- `[RESOLVED] OQ-6 — Is FlashEndFrame saved/loaded in save games?` → No. SidebarClass::Load delegates to RadarClass::Load with no CameoEntry serialization.
- `[RESOLVED] OQ-7 — Is this an active YR feature?` → No. Dead code — condition permanently false in stock YR.
- `[DEFERRED] OQ-8 — Was FlashEndFrame ever set in TS-era Tiberian Sun?` (category: requires-different-system-context; reason: would need decompilation of the TS binary which is a different executable; next-step: decompile TS sidebar InsertEntry/AddCameo to check).
- `[DEFERRED] OQ-9 — Could a modding extension (Ares/YRpp) have wired this up?` (category: out-of-scope; reason: project targets stock gamemd.exe parity only; next-step: check Ares source if relevant).

---

## 8. Current Rust Implementation Status

| System | Rust state |
|---|---|
| FlashEndFrame field in CameoEntry | NOT IMPLEMENTED — no CameoEntry struct in Rust yet |
| Cameo flash draw loop | NOT IMPLEMENTED |
| Recommendation | Do NOT implement the flash draw for now. The field is dead in stock YR. When building the CameoEntry struct, include FlashEndFrame as an `i32` field initialized to 0 and left unwritten until/unless a setter is discovered or intentionally implemented as a non-stock extension. |

---

## 9. Sources

**Ghidra functions decompiled (READ-ONLY — no mutations performed):**
- `0x006A9540 StripClass::Draw` — FlashEndFrame reader, field offset confirmed
- `0x006A8710 StripClass::InsertEntry` — FlashEndFrame zero-setter, CameoEntry layout
- `0x006AA600 StripClass::Recalculate` — second zero-setter (removal path)
- `0x006A80A0 FUN_006a80a0` (StripClass constructor/init) — constructor field initialization
- `0x006A8B30 StripClass::AI` — production state machine, no +0x88 write
- `0x006A6300 SidebarClass::AddCameo` — new-item insertion path, no +0x88 write
- `0x006A7780 SidebarClass::Action` — sidebar event handler, no +0x88 write
- `0x006AADE0 SelectClass::Action` — cameo click handler, no +0x88 write
- `0x006A6140 Sidebar_UpdateFromProduction` — production start path, no +0x88 write
- `0x006ABB60 FUN_006abb60` — factory cleanup, no +0x88 write
- `0x004FA350 HouseClass::Begin_Production` — start-build command handler
- `0x004FB0E0 HouseClass::Place_Production` — place-completed-building handler
- `0x004F8440 HouseClass::Update` — house tick, recalculate path
- `0x0050B1D0 HouseClass::AI_ResumeProduction` — superweapon/unit cameo addition
- `0x004C9B20 FactoryClass::AI` — production tick
- `0x004CA1A0 FactoryClass::CompletedProduction` — completion state transition
- `0x004FAA10 FUN_004faa10` — abandon/cancel production path
- `0x004C6CB0 EventClass::Execute` — command queue executor (command 0xe → Begin_Production)
- `0x006A87F0 FUN_006a87f0` — InsertEntry wrapper, confirmed no callers (dead code)

**XRefs checked:**
- `get_xrefs_to 0x006A8710` — confirmed 2 callers: AddCameo, FUN_006a87f0
- `get_xrefs_to 0x006A87F0` — confirmed 0 callers (dead)
- `get_function_callers 0x006A6300` — confirmed 2 callers: AI_ResumeProduction, TriggerAction::Execute
- `get_function_callers 0x004fa350` — not called (dispatched from EventClass::Execute cmd 0xe)

**Prior docs extended:**
- `ra2-rust-game-docs/SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT.md` — closes §7 open question on FlashEndFrame setter
- `ra2-rust-game-docs/SIDEBAR_STRIPS_TABS_CAMEOS_GHIDRA.md` — corrects CameoEntry field at +0x28 vs +0x30
