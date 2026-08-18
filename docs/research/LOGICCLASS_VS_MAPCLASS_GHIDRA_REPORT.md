# LogicClass vs MapClass — Role Distinction & Per-Tick Object Loop

**Date:** 2026-04-22
**Confidence:** HIGH for tick ordering and function names (cross-checked via `Main_Tick` call sequence and existing MapClass report); MEDIUM for the `LogicClass__AI` rename recommendation (behavior verified, exact original-name confirmation absent).
**Active in YR:** Yes — these are the top-level game loop classes, always active.

## 0. Why this doc exists

A previous session labeled `MapClass` as "world container, per-tick iteration, object
lists, logic tick loop." That description mixes two different classes. The existing
`MAPCLASS_GHIDRA_REPORT.md` + follow-up (1100+ lines) already documents MapClass's
actual responsibilities (cell grid, shroud, bridges, zones, crates, render-layer
display hierarchy). What was missing is a **LogicClass** report to clear up the
confusion. This document fills that gap.

**TL;DR:**
- **MapClass** = cell-grid + display hierarchy root. Owns world *geometry* (cells,
  bridges, shroud, zones, crates) and is the base of the big `GScreenClass ↔
  SidebarClass` single-inheritance mega-class. Does **not** iterate the object list.
- **LogicClass** = per-tick object driver. A `LayerClass<ObjectClass*>` that holds
  active world objects and calls `ObjectClass::AI()` on each one every frame.
- **`Main_Tick`** (0x0055D360) is the real "game loop" function, and it calls *both*:
  input → `LogicClass__AI` (command dispatch, **mislabeled** — see §5) → `Map__Logic`
  (trigger tag highlight) → `RenderFrame` → `LogicClass__PerTickUpdate` (the actual
  per-tick object AI loop) → housekeeping.

---

## 1. Main tick entry point — `Main_Tick` @ 0x0055D360

This is the Westwood-style `MainLoop()` function, driven from the Win32 message
pump. Simplified trace of the per-frame call order:

```
Main_Tick():
    if !g_GameActive: return
    ...timing / network service boilerplate...

    if (_DAT_00a8d5f8 & 2) == 0 && g_GameState == 0 && g_GameRunning:
        GScreenClass__Input(&key, &mx, &my)     # 0x004F4320 — polls input
        LogicClass__AI(&key)                    # 0x0055DEE0 — command dispatch
        if DAT_00a8b8b4:
            House_AI_Tick()
        if (g_CurrentFrameCounter & 7) == 7 && g_GameMode == 4:
            Network_Keepalive()
        Map__Logic()                            # 0x004D2370 — trigger-tag highlight
        RenderFrame_main()

    ...save/load snapshot handling...
    FUN_00551a30()                              # message log update

    LogicClass__PerTickUpdate()                 # 0x0055AFB0 — THE per-tick loop
    ...scroll + cursor side effects...
    FUN_00637550()
    FUN_005d4430()
    ...timing accounting...
    Network_ServiceLoop()
    g_CurrentFrameCounter += 1
```

**Key implication:** `LogicClass` contributes to the tick at **two points**:
1. **Early** — input-phase command dispatch (`LogicClass__AI`)
2. **Late** — per-tick object AI update (`LogicClass__PerTickUpdate`)

MapClass contributes at **one point** — `Map__Logic` between `Network_Keepalive`
and `RenderFrame_main` — and that function is a free-standing helper, not a
MapClass method (see §6).

---

## 2. LogicClass — class layout

**Vtable:** `0x007E18FC` (64 slots, 256 bytes)
**RTTI:** `.?AVLogicClass@@` at `0x00816B38`
**Global instance:** `0x0087F778` — verified by reading the assembly at the sole
PerTickUpdate call site (Main_Tick 0x0055DC99): `MOV ECX, 0x87F778` immediately
before `CALL 0x0055AFB0`. Pre-runtime BSS content is all-zeros (standard C++
static object state before CRT construction).

**Inheritance:** `LayerClass<ObjectClass*>` — an alias of `DynamicVectorClass<ObjectClass*>`
with AI iteration semantics. The vtable destructor at slot 0 (`FUN_0040CC20`) falls
back to `PTR_FUN_007E192C` (the DynVec destructor) which confirms the DynVec-based
layout.

### Instance layout (inferred from `LogicClass__PerTickUpdate`)

| Offset | Size | Type | Field | Evidence |
|--------|------|------|-------|----------|
| +0x00 | 4 | ptr | vtable | `0x007E18FC` |
| +0x04 | 4 | ptr | data_ptr | ObjectClass*[] — the active object list. Accessed as `*(int *)(param_1 + 4) + iVar6 * 4` at 0x0055B5A5 |
| +0x08 | 4 | int | capacity | Standard DynVec |
| +0x0C | 1 | bool | owns_memory | Standard DynVec |
| +0x0D | 1 | bool | flag | Standard DynVec |
| +0x0E-0x0F | 2 | — | padding | |
| +0x10 | 4 | int | count | Active object count. Read as `*(int *)(param_1 + 0x10)` at 0x0055B592 (the loop bound) |
| +0x14 | 4 | int | grow_step | Standard DynVec |

**Total size:** 24 bytes (0x18) — a minimal LayerClass. Behavior lives entirely in
virtual methods and the two top-level entry points.

---

## 3. `LogicClass__PerTickUpdate` — the per-tick object AI loop

**Address:** `0x0055AFB0`
**Called from:** `Main_Tick` only — confirmed via `get_function_callers 0x0055AFB0`; only
one caller returned. Call instruction is at **0x0055DC9E** (not 0x0055DD01 as previously
noted; assembly at 0x0055DC99: `MOV ECX, 0x0087F778` then `CALL 0x0055AFB0` at 0x0055DC9E).
`FUN_0055E160`, `FUN_00648350`, `FUN_00648710` are callers of **`LogicClass__AI`** (0x0055DEE0),
not of PerTickUpdate — see Sources §for confirmation.
(corrected 2026-05-29: call site was "@ 0x0055DD01 (and three other spots …)" — binary shows call at 0x0055DC9E, callers via get_function_callers 0x0055AFB0 return only Main_Tick — OPERATOR_OR_ORDER_DRIFT / GHIDRA_ADDRESS_SHIFT)
**Confidence:** HIGH — full decompilation traced.

### What it drives (per-tick, in order)

Selected systems invoked by this function, in their literal call order. Globals
resolved via xref and call-site inspection — see §8 evidence table.

```
1.  DAT_00ABCD40 += 1                         # internal tick/subtick counter
2.  For DAT_008B40D8 iterations:              # trigger-action dispatch loop
      TriggerAction__WalkChain(actionID=0x32|0x1b|0x1c|0x24|0x25|...,
                               ???,
                               subject=DAT_00ABCCD8, 0, 0)
      (function at 0x006E53A0 — currently mislabeled TechnoClass__ProcessCellAction;
       actually a TriggerActionEntry linked-list walker that fires EvaluateConditions
       + PlayVoiceForObjects + action dispatch. Gated by scenario flags at
       g_Scenario (DAT_00A8B230) +0x34AA/+0x34AB/+0x34BE.)
3.  RulesClass flag 0x17F0 branch:            # per-tick tiberium growth driver
      ftol() → FUN_004ACAC0()
4.  if (frame % 120 == 0):
      MapClass::RecalcBridgeShroudFlags()
5.  SpecialFlags & 0x1000 branch:             # fog of war (TS-legacy, disabled in YR)
      ftol() → FUN_004ACBC0()
6.  Tiberium spread driver                    # RulesClass +0x1668 gated
      + ore-frame-sound play FUN_004AE4C0()
7.  TiberiumClass::GrowthDriver_AllTypes()
8.  TiberiumClass::SpreadDriver_AllTypes()
9.  BombClass::UpdateAll()
10. FUN_0054E4D0()                            # 30-frame-periodic scripted
                                              # object-action trigger (reinforcement
                                              # / team-event cadence — see §5a)
11. Local DynVec filter + TeamClass::AI iteration
      over `g_TeamClass_Array[g_TeamClass_Array_Count]` (TEAM OBJECTS — not a generic
      object pool): filtered into a local DynVec (capacity 10, `local_4=10`) using
      `FUN_0055bb40`, then for each item: obj->vtable[0x5C]()
      (corrected 2026-05-29: was "DAT_008B40EC[DAT_008B40F8] (GLOBAL OBJECTS POOL)" — binary
      decompile of 0x0055AFB0 shows `g_TeamClass_Array / g_TeamClass_Array_Count` with local
      DynVec filter, no reference to DAT_008B40EC/DAT_008B40F8 — RTTI_LABEL_DRIFT)
12. g_DiskLaserClass_Array[] AI iteration
13. FUN_005FF390()                            # age-based object reaper — iterates
                                              # DAT_00AC167C pool, ages each +8/tick,
                                              # at >79 calls vtable+0x10 (Remove)
                                              # and operator delete (§5b)
14. LaserDrawClass::UpdateAllAI()
15. LightningStorm::Process()
16. DAT_00B04BD4[DAT_00B04BE0] AI iteration   # RadSiteClass pool (radiation sites)
                                              # verified via RadSiteClass__Constructor xref
17. FUN_00554D50()                            # incremental cell attribute recalc —
                                              # time-budgeted batch processor over
                                              # DAT_00ABCA44[DAT_00ABCA50] (§5c)
18. EMPulseClass::UpdateAll()
19. THIS LogicClass instance's own layer:
      for i in 0..(this[0x10]): this[0x04][i]->vtable[0x5C]()
20. If g_GameMode not 0/5:
      DAT_00A83E04[DAT_00A83E10] AI iteration # AnimClass pool (active animations)
                                              # verified via AnimClass__Constructor xref.
                                              # Skipped in editor/intro mode.
21. FUN_0053D310()                            # wave splash forces tick —
                                              # iterates DAT_00AA0128 times, each
                                              # calls Wave_splash_forces()
22. AlphaShapeClass::PurgeDisabled()
23. MapClass::UpdateCrateRegenTimers()        # <-- MapClass invoked here, once per tick
24. g_Tactical->vtable[0x5C]()                # TacticalClass AI (camera/scroll)
25. g_FactoryClass_Array[] AI iteration
26. g_HouseClass_Array[] AI iteration
27. Last-refocused object: recenter tactical view on it
28. Free local DynVec buffer
```

**Calling convention on step 11/19:** `vtable+0x5C` is the `ObjectClass::AI()` slot.
Every object in the pool gets its virtual `AI()` method invoked here. This is the
point where units move, buildings produce, projectiles fly, etc.

**Step 11 vs step 19** — two distinct object sets are iterated:
- **Step 11** iterates the *TeamClass pool* at `g_TeamClass_Array /
  g_TeamClass_Array_Count` (TEAM OBJECTS — not a generic object pool), filtered
  into a temp DynVec built with `FUN_0055BB40` (`DynamicVectorClass<ObjectClass*>`
  constructor at 0x0055BB40, capacity 10 / `local_4 = 10`). The copy shields
  against mid-iteration mutation; each surviving item then gets `vtable[0x5C]()`.
  There is **no** read of `DAT_008B40EC / DAT_008B40F8` in this function.
  (corrected 2026-05-29: was "global object pool at DAT_008B40EC / DAT_008B40F8"
  — decompile_function 0x0055AFB0 shows the loop bound is `g_TeamClass_Array_Count`
  and the data source is `g_TeamClass_Array + iVar9 * 4`, with no reference to
  DAT_008B40EC/DAT_008B40F8 — RTTI_LABEL_DRIFT)
- **Step 19** iterates *this LogicClass instance's own* layer at `this[0x04]`
  (`param_1 + 4`) for `this[0x10]` (`param_1 + 0x10`) entries. Same vtable slot
  0x5C. This is the only generic-ObjectClass layer in the function.

This is the Westwood pattern: the "Logic" singleton owns its own object layer
(step 19), iterated alongside the dedicated subsystem pools (Teams, DiskLasers,
RadSites, Anims, Factories, Houses, etc.).

**Step 23 — the only direct MapClass call in the whole loop** — confirms MapClass
is a *data* dependency of the logic tick, not the *driver*.

### 3a. Per-tick helper functions resolved

**FUN_0054E4D0 — 30-frame scripted-action trigger (step 10).**
Takes a timer struct with fields `{last_frame, _, period=30, data_ptr, _, _, _, count}`.
Every 30 frames iterates entries, each with `(obj_ptr, mode)`:
- Sets `obj[0xBF] = 1` (some state flag)
- If mode == 0: picks random direction via `RateTimer::Current` hash, calls obj
  vtable+0x1BC (rotate?)
- Unconditionally calls obj vtable+0x3C8 (move?) and vtable+0x1E8(1, 0) (mark?)

Purpose inferred: **periodic scripted behavior queue** — reinforcement spawning,
team AI state progression, or similar time-gated script actions. Not a free-running
AI driver, just a cadence-gated dispatcher.

**FUN_005FF390 — age-based object reaper (step 13).**
Walks `DAT_00AC167C[DAT_00AC1688]` (a pointer array) backwards. Each entry has a
counter at `+0xC`; counter += 8 per tick. When counter > 79 (≈10 ticks of aging):
1. Call vtable+0x10 on `DAT_00AC1678` (container) to remove the entry
2. Shift-down the rest of the array
3. `operator delete` the entry

Purpose: **FX/debris cleanup pool** — short-lived visual entities with ~80-tick
lifetime (bullet splashes, bounce trails, or similar). The container at
`DAT_00AC1678` owns them.

**FUN_00554D50 — incremental cell attribute recalc (step 17).**
Time-budgeted batch processor. Takes `(time_budget_ms, finalize_flag)`. Operates
on `DAT_00ABCA44[DAT_00ABCA50]` — an array of pending cell-update records.

Per-record behavior:
- If the record's first dword is 0 (unprocessed): resolve cell via
  `MapClass::Get_CellClass`, call `FUN_00484050` to compute new cell state,
  stash six shorts back into the record.
- On each iteration, checks elapsed time via `FUN_005B1E40` (hi-res timer).
  If elapsed > `time_budget_ms`, yields control for the frame.
- When all records processed (flag `DAT_00ABCA84` set): walks the array again,
  calls `FUN_00483E30` per cell (commit), frees records, releases the array.

Purpose: **INI-reload / rules-change cell refresh** — after `[General]`,
`[Map]`, or rule changes, cell attributes need re-derivation. This function
amortizes that across multiple frames to avoid hitching. Ratelimited further
via `DAT_00829AE8` (adaptive tick skip).

**FUN_0053D310 — wave splash forces tick (step 21).**
Trivial: for `DAT_00AA0128` iterations, calls `Wave_splash_forces()`. This
advances the wave-physics simulation for shoreline/ocean splash effects.

---

## 4. MapClass during the tick — what it actually does

MapClass has no `AI()` or `PerTickUpdate()` method in its vtable (64 slots dumped
in `MAPCLASS_GHIDRA_REPORT_FOLLOWUP.md` §3 — none are per-tick drivers). Its
per-tick contributions happen **on demand** from `LogicClass__PerTickUpdate` or from
specific events:

| Method | Address | Called from | Frequency |
|--------|---------|-------------|-----------|
| `UpdateCrateRegenTimers` | 0x0056BBE0 | `LogicClass__PerTickUpdate` step 23 | Every tick |
| `RecalcBridgeShroudFlags` | 0x00578100 | `LogicClass__PerTickUpdate` step 4 | Every 120 frames |
| `RevealShroud` | 0x005673A0 | `TechnoClass::RevealToHouses`, etc. | Event-driven (unit movement, sensor update) |
| `Viewport_Resized` (slot 29) | 0x00567230 | Tactical clip rect change | Event-driven (scroll) |
| `UpdateBridgeZonesHelper` | 0x0056C510 | Bridge destroy/repair | Event-driven |
| `SetOverlayAndPropagate` | 0x0056EB80 | Bridge damage state machine | Event-driven |
| `AssignOrphanedCellZone` | 0x0056D460 | Cell mutation (building remove) | Event-driven (fast path) |
| `MergeAdjacentCellZone` | 0x0056D5A0 | Cell mutation (terrain change) | Event-driven (fast path) |

**MapClass is passive data** — it provides the world geometry that LogicClass
iterates. It does not drive the tick.

---

## 5. `LogicClass__AI` — the mislabeled function

**Address:** `0x0055DEE0`
**Confidence:** HIGH — verified via assembly at all four call sites.

### 5.1 Call-site evidence (the decisive finding)

All four callers pass `ECX = &stack_keycode`, NOT a LogicClass instance:

| Caller | Site | ECX setup before CALL |
|--------|------|-----------------------|
| Main_Tick | 0x0055D8B4 | `LEA ECX, [ESP + 0x38]` — stack local |
| FUN_0055E160 | 0x0055E25C | `LEA ECX, [ESP + 0x10]` — stack local |
| FUN_00648350 | 0x006485C0 | `LEA ECX, [ESP + 0x20]` — stack local |
| FUN_00648710 | 0x00649833 | `LEA ECX, [ESP + 0xB0]` — stack local |

In every case the pattern is identical: `MOV ECX, 0x87F7E8` (Map singleton) is
used as `this` for the prior `GScreenClass::Input` call (0x004F4320), which fills
the stack keycode. Then `LEA ECX, [ESP+N]` loads the address of that keycode,
and the CALL to 0x0055DEE0 happens.

A true `LogicClass::AI` method would receive `ECX = 0x87F778` (the LogicClass
singleton verified in §2). It does not. This function is not a method on
LogicClass — it operates on a heap/stack-addressed `uint*` that happens to be a
keyboard key code.

### 5.2 Decompiled behavior

The existing label suggests this is the per-tick AI loop. It is not. Decompiled
behavior:

```
LogicClass__AI(uint *keycode_ptr):
    FUN_0055E420(keycode_ptr)           # handles Enter/Backslash/Backspace chat keys
    raw_key = *keycode_ptr
    if raw_key == 0: return
    stripped_with_bit11  = raw_key & 0xFFFFF7FF   # mask out bit 11
    stripped_modifiers   = raw_key & 0xFFFFE0FF   # mask out modifier bits 8..12

    # Two-probe CommandClass hash lookup:
    cmd = hash_find(key_table, stripped_modifiers)
    if cmd == null:
        cmd = hash_find(key_table, stripped_with_bit11)

    if cmd != null:
        if cmd->vtable[0x18](raw_key):       # CommandClass::Execute_Allowed?
            cmd->vtable[0x20](raw_key)       # CommandClass::Execute
        if cmd->vtable[0x1C](raw_key):       # CommandClass::Post_Execute_Allowed?
            # Auto-repeat while held (up to 10 iterations)
            while FUN_0054F000() & 0xFFFF == raw_key && n < 10:
                FUN_0054F050()               # auto-repeat tick
    else:
        # Hard-coded fallbacks for specific keys:
        # 0x1B → FUN_00647040 (ESC — menu/cancel)
        # 0x09 → FUN_006ABC40 (TAB — toggle radar/sidebar?)
        # 0x25..0x28 → DAT_00ABCE14 scroll bit flags (arrow keys)
        ...
```

**What it actually is:** the **keyboard command dispatcher**. It receives a
pressed-key event from `GScreenClass__Input` and dispatches it to a `CommandClass`
instance registered in the global key-map hash table (`DAT_0087F684`/`DAT_0087F690`,
with `FUN_0055F6E0` as the full hash-probe fallback).

**The vtable slots +0x14/+0x18/+0x1C/+0x20 are CommandClass methods**, not
ObjectClass::AI. This matches the Westwood `CommandClass` hierarchy used in
RA2/TS for hotkey bindings.

**Recommendation:** rename `LogicClass__AI` → `LogicClass__ProcessKeyCommand` or
`CommandClass__DispatchKey`. Do NOT confuse this with a per-tick AI loop. Not
renaming in this pass per project policy — flagging as an open rename for the
next annotation sweep.

The "LogicClass__" prefix may just be Ghidra's guess from address proximity; the
actual owner could be a `KeyboardCommands` or `CommandDispatcher` class. Ghidra
labels are not ground truth — see CLAUDE.md "Ghidra annotation best practices".

---

## 6. `Map__Logic` — NOT a MapClass method

**Address:** `0x004D2370`
**Confidence:** HIGH

Despite the name, this is a **free-standing global function**, not a method on
MapClass. It walks `DAT_008B3D14[DAT_008B3D20]` — an array of trigger/tag or
waypoint objects — and for each:

- If the object type is 6 (tag/area trigger): expand its associated point list
  and for each cell in the list, OR flag `0x400000` into `cell.flags[0x140]`.
- Otherwise: look up its single cell and OR flag `0x400000`.

**Purpose:** per-tick **trigger tag highlight marker**. Cell flag bit 22
(`0x400000`) is set on cells that belong to an active tag area — used later in
the frame by the tactical renderer to draw the yellow tag outlines visible in
skirmish/campaign missions.

**It is not a MapClass method.** The name is misleading; it should be something
like `TagClass__Highlight_All_Cells`. The only MapClass involvement is that it
calls `MapClass__Get_CellClass` (0x5657A0) to resolve cell coordinates.

---

## 7. Current Rust implementation status

### Per-tick driver

The Rust equivalent is `World::advance_tick` in `src/sim/world.rs`, which matches
the LogicClass responsibility — iterate all active entities and advance their
state once per frame. Tick-phase order per CLAUDE.md:

```
commands → ground movement → air + special movement → vision →
power → turrets + combat → retaliation + passengers →
scatter + production + repairs + docks + ore growth → AI →
defeat detection → building anims + cleanup → state hash
```

This is a **richer, better-structured** version of LogicClass — the binary runs a
single mega-AI pass (step 11/19), whereas the Rust engine breaks it into
deterministically-ordered sub-phases. Good design choice; the sub-phases aren't
directly comparable to binary call-order.

### Coverage matrix

| Binary concept | Rust equivalent | Status |
|----------------|-----------------|--------|
| `LogicClass` DynVec of active objects | `EntityStore` (`BTreeMap<u64, GameEntity>`) | Implemented |
| `LogicClass__PerTickUpdate` | `World::advance_tick` | Implemented |
| `LogicClass__AI` (command dispatch) | `src/sim/command_queue.rs` / player input layer | Implemented — different approach (command queue, not hotkey hash) |
| Step 11 `g_TeamClass_Array` pool (TeamClass objects) + step 19 LogicClass own layer (`param_1[0x04]`) | `EntityStore` (teams not modelled as a separate AI pool yet) | Step 19 layer = Implemented; TeamClass pool = Not implemented (corrected 2026-05-29: row previously read "Global object pool at DAT_008B40EC — Same EntityStore" — decompile_function 0x0055AFB0 shows no DAT_008B40EC read; step 11 iterates g_TeamClass_Array, step 19 the instance layer — RTTI_LABEL_DRIFT) |
| `TechnoClass__ProcessCellAction` queue | — | **Not implemented** — needs investigation; may be cell-entry/exit events |
| `MapClass` cell grid | `src/map/resolved_terrain.rs` + `src/sim/*` | Implemented (see MAPCLASS_GHIDRA_REPORT.md §6) |
| `Map__Logic` trigger tag highlight | — | **Not implemented** — trigger system not ported yet |
| 120-frame `RecalcBridgeShroudFlags` | `src/sim/bridge_state.rs` | Partially implemented — cadence unknown; audit needed |
| `UpdateCrateRegenTimers` | — | **Not implemented** — crate system missing |

### Architectural note

The Rust engine's invariant — `sim/` never depends on `render/`/`ui/`/`sidebar/`
— **cannot be cleanly mirrored** from the binary's class layout, because
gamemd.exe intentionally fuses the world grid with the render/UI stack via
single inheritance:

```
GScreenClass → MapClass → DisplayClass → RadarClass → PowerClass → SidebarClass → …
```

The global `Map` instance at `0x0087F7E8` is ONE 21,868-byte object that is the
cell grid *and* the tactical display *and* the sidebar. This is why a click on
"the map" and a click on the radar dispatch through the same vtable — they're
the same object.

**For Rust parity, do not port that fused hierarchy.** The logical split we
already have is correct; just be aware that binary behavior "stored on Map" may
actually belong conceptually to any of those six layers.

---

## 8. Confidence & evidence table

| Finding | Evidence | Confidence |
|---------|----------|------------|
| `Main_Tick` is at 0x0055D360 and is the per-frame entry | Decompilation + xref from Win32 message pump | HIGH |
| `LogicClass__PerTickUpdate` is the real object AI loop | Full decompilation; iterates 11+ systems including vtable+0x5C calls | HIGH |
| **Global LogicClass instance = 0x0087F778** | Assembly at 0x0055DC99: `MOV ECX, 0x87F778` immediately before `CALL 0x0055AFB0` | HIGH (verified) |
| `LogicClass__AI` is NOT a LogicClass method | Assembly at all 4 call sites: ECX = `LEA ECX,[ESP+N]` (stack keycode), not 0x0087F778 | HIGH (verified) |
| `LogicClass__AI` dispatches keyboard commands | Decompilation shows CommandClass-style vtable+0x18/+0x20 dispatch on masked key codes | HIGH |
| `Map__Logic` is a free-standing trigger-tag highlighter | Decompilation; iterates `DAT_008B3D14`, not a MapClass global | HIGH |
| LogicClass inherits LayerClass/DynVec<ObjectClass*> | Vtable destructor falls back to `PTR_FUN_007E192C` (DynVec vtable); instance layout matches DynVec | HIGH |
| MapClass is passive data, not a tick driver | No AI/Update method in 64-slot vtable; only one MapClass call in PerTickUpdate (UpdateCrateRegenTimers) | HIGH |
| MapClass's actual role (cell grid, shroud, bridges, zones, crates) | Pre-existing MAPCLASS_GHIDRA_REPORT.md — 25+ functions decompiled | HIGH |
| **DAT_00B04BD4 = RadSiteClass pool** | Xref from `RadSiteClass__Constructor` (READ + WRITE at 0x0065B167/0x0065B1C7) | HIGH (verified) |
| **DAT_00A83E04 = AnimClass pool** | Xref from `AnimClass__Constructor` (0x00422A9E READ) + FootClass__ClickedAction_Cell | HIGH (verified) |
| **DAT_00A8B230 = ScenarioClass instance** | Xrefs from `Main_Game`, `State_Machine`, `Network_ServiceLoop` (all scenario lifecycle) | HIGH (verified) |
| **DAT_00ABCCD8 = current trigger subject pointer** | Used as arg 3 to all 10 trigger-action calls; not iterated (static-ish per subtick) | MEDIUM |
| **FUN_006E53A0 is mislabeled** (currently TechnoClass__ProcessCellAction) | Body iterates linked list via +0x24/+0x28 and calls `TriggerActionEntry__EvaluateConditions` / `TriggerActionEntry__PlayVoiceForObjects` — it's a trigger-action chain walker | HIGH (verified) |
| The numeric IDs 0x0D/0x0E/0x1B/0x1C/0x24/0x25/0x2D/0x2E/0x32/0x33 are TriggerAction types | They index into the trigger system; exact name mapping requires the TriggerActionID enum | MEDIUM |
| **FUN_005FF390** is an age-based FX reaper (~80-tick lifetime) | Decompilation shows counter at +0xC incremented by 8/tick, threshold 0x4F, then delete | HIGH |
| **FUN_00554D50** is incremental cell attribute recalc (time-budgeted) | Decompilation shows MapClass::Get_CellClass + FUN_00484050 + time check via FUN_005B1E40 | HIGH |
| **FUN_0053D310** is wave-splash forces tick | Decompilation: loop N times calling `Wave_splash_forces()` | HIGH |
| **FUN_0054E4D0** is 30-frame scripted-action trigger | Decompilation shows 30-frame period (`param_1[2] = 0x1E`) + vtable+0x1BC/+0x3C8/+0x1E8 dispatch | HIGH |
| The two iterated object sets at step 11 and step 19 are distinct | Step 11 = `g_TeamClass_Array` (global Teams pool, filtered into local DynVec); Step 19 = `param_1[0x04]` (this LogicClass instance layer); confirmed via decompile_function 0x0055AFB0 (corrected 2026-05-29: was "DAT_008B40EC vs param_1[0x04]" — binary shows g_TeamClass_Array for step 11 — RTTI_LABEL_DRIFT) | HIGH |

---

## 9. Open questions

All five open questions from v1 resolved. Remaining lower-priority items:

1. **Exact TriggerActionID name mapping.** The 10 numeric IDs (0x0D, 0x0E, 0x1B,
   0x1C, 0x24, 0x25, 0x2D, 0x2E, 0x32, 0x33) are confirmed to be
   TriggerActionEntry type codes, but the enum names would require decompiling
   the trigger dispatch switch (address unknown) or the INI trigger action parser
   — neither needed for current Rust parity work since triggers aren't ported
   yet.

2. **Step 2 outer loop semantics.** `DAT_008B40D8` is the outer loop bound
   (iterated with counter `DAT_00A83CDC`) but the counter isn't used in the body.
   Unclear if the loop is "try each subject in turn" (with `DAT_00ABCCD8`
   mutating somewhere we haven't traced) or "repeat per pending-queue entry"
   (with the queue drained elsewhere). Not blocking.

3. **Rename recommendations for next annotation sweep** (do NOT apply blindly,
   per CLAUDE.md Ghidra policy — each needs verification with current context):
   - `LogicClass__AI` → `CommandClass__DispatchKey` (high confidence — behavior
     verified across 4 call sites)
   - `TechnoClass__ProcessCellAction` → `TriggerActionEntry__WalkChain` (high
     confidence — body confirmed trigger-based, not Techno-based)
   - `Map__Logic` → `Scenario__Mark_TagArea_Cells` or
     `TagClass__Highlight_Cells` (medium — need to confirm against scenario
     trigger terminology)

4. **FUN_005FF390 container identity.** `DAT_00AC1678` (the container) and
   `DAT_00AC167C` (its data array) drive the 80-tick-lifetime reaper. Type
   unknown — could be ParticleClass, BulletClass, or some debris subtype.
   Decompiling the constructor of whatever writes to these globals would
   resolve it.

5. **FUN_00554D50 ratelimit details.** The adaptive skip via `DAT_00829AE8`
   (set to either 0x32 = 50 or 0x32-0x30 = 2 based on `FUN_00544FF0` result)
   isn't fully understood. Low priority.

---

## 10. Action items for Rust parity

Nothing urgent — the Rust engine's `World::advance_tick` already matches
LogicClass's role, and its structure (explicit sub-phases) is cleaner than the
binary's single flat loop. Items to validate opportunistically:

- Confirm `RecalcBridgeShroudFlags` cadence (every 120 frames) is honored by
  whichever Rust module does bridge shroud updates.
- Confirm `UpdateCrateRegenTimers` runs every tick (not every N ticks) when/if
  the crate system is ported.
- If trigger tags are ported: port `Map__Logic` as a per-tick cell flag painter
  (flag bit 22 / 0x400000 in gamemd; equivalent render bit in Rust cell state).

---

## Sources

### Ghidra addresses decompiled (11)

- **0x0055D360** — `Main_Tick` (full per-frame entry, ~500-line decompilation)
- **0x0055AFB0** — `LogicClass__PerTickUpdate` (the real per-tick AI loop)
- **0x0055DEE0** — `LogicClass__AI` (command dispatcher — mislabeled)
- **0x004D2370** — `Map__Logic` (trigger tag highlight — not a MapClass method)
- **0x0055BB40** — `DynamicVectorClass<ObjectClass*>` ctor (confirms temp buffer
  in step 11 of PerTickUpdate)
- **0x0040CC20** — LogicClass scalar deleting destructor (confirms DynVec inheritance)
- **0x006E53A0** — mislabeled `TechnoClass__ProcessCellAction` (actually a
  `TriggerActionEntry` linked-list walker)
- **0x005FF390** — age-based FX reaper (~80-tick lifetime)
- **0x00554D50** — incremental cell attribute recalc (time-budgeted)
- **0x0053D310** — wave-splash forces tick
- **0x0054E4D0** — 30-frame-periodic scripted-action trigger

### Assembly traced (2 sites)

- **0x0055DC99** — `MOV ECX, 0x0087F778` (5 bytes) then `CALL 0x0055AFB0` at 0x0055DC9E → confirms LogicClass singleton address and PerTickUpdate call site (corrected 2026-05-29: call site previously noted as 0x0055DD01 — binary shows CALL at 0x0055DC9E — GHIDRA_ADDRESS_SHIFT)
- **0x0055D8B4, 0x0055E25C, 0x006485C0, 0x00649833** — all four callers of
  `LogicClass__AI` (0x0055DEE0) pass `LEA ECX, [ESP+N]` (stack keycode), not
  a LogicClass instance

### Raw memory reads

- **0x007E18FC** (256 bytes) — LogicClass vtable
- **0x0087F778** (24 bytes) — LogicClass singleton (all zeros in BSS)

### Global xrefs checked

- `DAT_00B04BD4` → `RadSiteClass__Constructor` (confirms RadSite pool)
- `DAT_00A83E04` → `AnimClass__Constructor` (confirms Anim pool)
- `DAT_00A8B230` → `Main_Game`, `State_Machine`, `Network_ServiceLoop` (confirms
  ScenarioClass singleton)
- `DAT_008B40D8`, `DAT_008B40EC`, `DAT_00ABCCD8` → all LogicClass__PerTickUpdate
  internal reads (trigger subject + pool counters)
- `LogicClass` RTTI string (0x00816B38) → 0 xrefs (RTTI emitted but never
  referenced via typeid())
- `LogicClass` vtable (0x007E18FC) → `list_globals` (`vtable__LogicClass` label)
- `LogicClass__PerTickUpdate` callers → `Main_Tick` only (the other three were
  verified to be `LogicClass__AI` callers, not PerTickUpdate)

### Existing docs referenced

- `MAPCLASS_GHIDRA_REPORT.md` — struct layout, inheritance, zones, bridges, shroud
- `MAPCLASS_GHIDRA_REPORT_FOLLOWUP.md` — vtable enumeration, Init_Clear,
  viewport resize, coord transforms
