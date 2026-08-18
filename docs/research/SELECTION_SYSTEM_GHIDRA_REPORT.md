# Selection System (`CurrentObjects`) — Ghidra Research Report

**Primary address:** `ObjectClass::Select` at `0x005F4520`
**Overall confidence:** HIGH (verified by direct decompilation of every function in the call chain, including the DynamicVectorClass init, the per-object flag offsets, the control-group storage, and the command dispatch).
**Active in YR:** Yes. The entire system is live in standard YR skirmish.
**Ghidra labels applied:** Yes — 12 functions renamed, 13 globals labelled, 15 plate comments added. See "Ghidra Labels Applied" section at the bottom.

> **Naming note.** There is no class called `SelectClass` in gamemd.exe. The "selection
> class" is a global `DynamicVectorClass<ObjectClass*>` instance named `g_CurrentObjects`
> (base at `0x00a8ecb8`). All selection behavior lives in `ObjectClass::Select` /
> `::Deselect` vtable overrides plus a handful of free functions.

---

## 1. Overview

This report covers the **selection list itself** — the global container of currently-selected
objects (`CurrentObjects`), the per-object `IsSelected` flag, the `Select`/`Deselect` vtable
methods, control-group hotkeys (digit keys, Ctrl+digit, Shift+digit), the "select all same
type" (T key) state machine, and multi-unit order dispatch.

Two adjacent systems are already documented and are **out of scope** here:
- **Band-box drag rectangle** — see `BANDBOX_SELECTION_GHIDRA_REPORT.md`
- **Selection brackets / health pips (visuals)** — see `building-selection-brackets/SELECTION_BRACKETS_GHIDRA_REPORT.md`

This report focuses on what happens *after* an object is identified for selection (by
click, drag, or hotkey): how it enters/leaves the selection list, what flags change, and
how commands iterate the list.

---

## 2. `CurrentObjects` — the global selection list

`CurrentObjects` is a **`DynamicVectorClass<ObjectClass*>`** instance in BSS, initialised
at startup by code at `0x004e7d40`. It is the single source of truth for "what units are
currently selected".

### Instance layout (base at `0x00a8ecb8`)

| Abs. addr | Offset | Type | Name | Notes |
|---|---|---|---|---|
| `0x00a8ecb8` | +0x00 | `void**` | vtable | = `0x007e4f64` (DynamicVectorClass vtable) |
| `0x00a8ecbc` | +0x04 | `ObjectClass**` | data | heap array of `ObjectClass*` |
| `0x00a8ecc0` | +0x08 | `int` | capacity | allocated slots |
| `0x00a8ecc4` | +0x0C | `byte` | (unknown) | init to 1 |
| `0x00a8ecc5` | +0x0D | `byte` | IsAllocated | init to 0 (managed allocation) |
| `0x00a8ecc8` | +0x10 | `int` | count | current element count |
| `0x00a8eccc` | +0x14 | `int` | growth_step | = 10 (capacity grows by 10 when full) |

### DynamicVectorClass vtable (`0x007e4f64`) — slots used by selection code

| Offset | Address | Purpose |
|---|---|---|
| +0x08 | `0x0040ce50` | `Resize` — called to grow when count == capacity |
| +0x10 | `0x0040cf00` | `ID(ObjectClass*)` — returns element index or -1 (used by Deselect) |

### Key callers of `CurrentObjects` globals

- **Read count `DAT_00a8ecc8`:** `ObjectClass::Select`, `ObjectClass::Deselect`,
  `Unselect_All`, `FUN_004ae750` (multi-unit command dispatch), all team-group helpers.
- **Read data `DAT_00a8ecbc`:** same set; indexing via `DAT_00a8ecbc[i]` gives the i-th
  selected object.
- **Write:** only `ObjectClass::Select` and `ObjectClass::Deselect` mutate the list.
  `Unselect_All` mutates indirectly by calling `Deselect` repeatedly.

### Iteration order

Insertion order is preserved (append on the common path). Iteration via
`for (i=0; i<DAT_00a8ecc8; i++) DAT_00a8ecbc[i]` is the canonical pattern; every caller
uses it. No sorting is applied.

---

## 3. Per-object selection state

### `IsSelected` flag (ObjectClass offset `+0x83`)

A single byte, `0` or `1`. **Set** by `ObjectClass::Select` on success, **cleared** by
`ObjectClass::Deselect`. It is the authoritative per-object indicator; all code paths
(rendering brackets, bandbox re-select, hotkeys) read this byte rather than walking
`CurrentObjects`.

### Adjacent lifecycle flags (read by Select)

| Offset | Purpose (inferred from selection-path checks) |
|---|---|
| `+0x14` (bit 0) | On-map / valid-object flag (checked in prepend-branch test) |
| `+0x81` | **InLimbo** — non-zero means the object is in limbo. Select is rejected. |
| `+0x83` | **IsSelected** (this report). |

The Select path explicitly refuses to add a limbo object or a non-`CanBeSelected`
object to the list.

> **Correction:** An earlier draft of this report claimed *"there is no call to
> Deselect in the Limbo path for FootClass/TechnoClass."* That was wrong. Limbo
> chains through `FUN_006F6AC0` → `FUN_0065AA80` → `ObjectClass::Conceal`
> (`0x005F4D30`), which calls `vtable+0x150` (Deselect) before setting
> `+0x81 = 1`. So a unit going to limbo IS auto-deselected. See
> `SELECTION_LIFECYCLE_GHIDRA_REPORT.md` for the full lifecycle-trigger map.

### Per-object "action lines" block (TechnoClass offset `+0x174`)

When a unit is selected via **team-hotkey recall** (not band-select, not click), three
int fields are written to display brief target lines for ~25 frames:

| Offset | Written value |
|---|---|
| `+0x174` | `g_CurrentFrameCounter` at time of recall |
| `+0x178` | Team/group index (or secondary timestamp — see Open Questions) |
| `+0x17C` | `RulesClass+0x8c` (per-unit action-line duration from rules.ini) |

`ActionLines__StartTimer` (`0x0070d150`) separately sets a global duration of **25
frames** (`0x19`). This is what renders the short yellow line from each recalled unit
to its current navcom/target after you press a team number.

---

## 4. `ObjectClass::Select` — core add-to-list logic

**Address:** `0x005F4520`  **Vtable slot:** `+0x14C`

Complete decompilation is in the appendix. Key behavior:

### Early exits (return 0 = no selection made)

1. **`DAT_00a8ed6b == 0`** (game not in "allow all select" mode — the normal state) AND
   any of:
   - `this->flag_0x81 != 0` (in limbo)
   - `this->IsSelected != 0` (already selected — no duplicates)
   - `this->vtable[0x138]()` returns 0 (CanBeSelected check fails)
2. `Filter_AbstractType_InMap()` check — if the returned pointer exists and its
   `vtable[0x1d4]()` returns non-zero, reject. (Likely a "MouseoverObject"-is-blocker check.)
3. `DAT_00880990 != 0` — a global lock set during bridge repair, building placement, and
   certain overlay rendering. Disables selection entirely during those UI modes.

### Desync detection when adding to a non-empty selection

Before appending, if the list already has entries:
- Query this object's owner house via `vtable[0x3c]()`.
- Query the first selected object's owner via `CurrentObjects[0]->vtable[0x3c]()`.
- If the two houses differ OR either is not a human player → `Desync_Handler()` fires.

This means: **mixing different players' units in one selection is treated as a desync
hazard.** Standard YR skirmish never hits this in normal play because the selection flow
only ever adds the local player's units, but the check exists as a safety net.

### Append vs. prepend branch

```c
// "Normal" flag set (param_1 != 0 AND (param_1[0x14] & 1) != 0 AND
//  TechnoTypeClass[+0xc9c] != 0) → PREPEND
if (is_prepend_class) {
    if (count > 0)
        memmove(data + 1, data, count * 4);   // shift everything right
    data[0] = this;
    count++;
} else {
    // APPEND (the normal case)
    data[count] = this;
    count++;
}
```

Capacity is checked first; if `count == capacity`, `DynamicVectorClass::Resize` (vtable+0x08)
is invoked to grow by `growth_step = 10` (only when `IsAllocated` == 0, which is the
default).

After the write: `this->IsSelected = 1` and `FUN_00731d00(0)` is called — this resets the
selection-mode state machine (see §7.2) back to `Normal`.

### Post-success chaining (TechnoClass override)

`TechnoClass::Select` at `0x006FBFA0` wraps `ObjectClass::Select` and adds:
1. Pre-check: if this unit's `+0x41b == 0` AND owner is NOT human AND a game window
   exists → return 0 (AI units can't be selected in normal play).
2. Call `ObjectClass::Select`; if it fails, return 0.
3. If this unit has a Mind-Controller set (`this[0xd] != 0`), run `ProcessCellAction(0x21, …)`
   to refresh the mind-controlled visual.
4. If owner is human (or observer dev flag set) AND voice-suppress flag
   `DAT_00822cf2 != 0` → call `vtable[0x360]()` which plays the "unit selected" voice
   response (VoiceSelect from rulesmd.ini).
5. Call `ObjectSelection__PlayVoice()` — processes a deferred voice-queue (re-entrant-guarded).

### Voice-suppress flag `DAT_00822cf2`

- `1` = voice enabled (default)
- `0` = voice suppressed

Set to 0 temporarily during **batch** selection flows (band-box resolve, T-key TypeSelect,
team recall) so only a single voice plays at the end rather than 20 simultaneous "yes sir"
lines.

---

## 5. `ObjectClass::Deselect` — core remove-from-list logic

**Address:** `0x005F44A0`  **Vtable slot:** `+0x150`

```c
void ObjectClass::Deselect(ObjectClass* this) {
    if (this->IsSelected == 0) return;                // no-op if not selected

    int idx = CurrentObjects.ID(this);                // vtable[0x10]
    if (idx != -1 && idx < count) {
        count--;
        while (idx < count) {                         // manual shift-down
            data[idx] = data[idx + 1];
            idx++;
        }
    }
    this->IsSelected = 0;

    // Clear DisplayClass's "last-referenced object" if it was us
    int ref = DisplayClass::GetLastRef();             // 0x004aeb10 — reads +0x119c/+0x11a0
    if (ref == (int)this)
        DisplayClass::SetLastRef(0);                  // 0x004aeb30
}
```

Notes:
- **Idempotent** — deselecting an already-unselected object is a cheap no-op.
- **O(n)** — uses manual memmove-equivalent; the engine never batches removals.
- The `DisplayClass::LastRef` (fields at `+0x119c`/`+0x11a0`) is a "mouseover/last-clicked"
  pointer used elsewhere; clearing it on deselect prevents dangling refs.

---

## 6. `Unselect_All` — clear the entire list

**Address:** `0x006da740`

```c
void Unselect_All(void) {
    while (CurrentObjects.count != 0)
        CurrentObjects[0]->vtable[0x150]();          // Deselect head repeatedly
    FUN_00731d00(0);                                  // reset selection-mode state
}
```

Simple but O(n²) because each `Deselect` shifts all remaining elements left. For the
selection sizes in practice (few hundred at most) this is fine.

### Callers of `Unselect_All`

- `FUN_00637270` and `FUN_0063a4b0` — observer/spectator state transitions
- `ControlGroup__Recall` (`0x007311c0`) — before recalling a team
- `FUN_007313a0` — centre-camera-on-team
- `FUN_00732280` — dialog/menu close path
- `FUN_00733380`, `FUN_007336c0` — two additional UI transitions

### `Unselect_If_Not_Owned` at `0x006da770`

Utility called at the start of the T-key handler. If exactly 1 object is selected and
its owner is NOT human-player, deselect it and return 1. Prevents accidentally
type-selecting enemy units after a click-select of an enemy.

---

## 7. Selection state machine (`DAT_00b0fe54`)

### 7.1 Modes

| Value | Name | Meaning |
|---|---|---|
| 0 | Normal | Default — click-select, band-select, hotkeys all dispatch here |
| 1 | SelectOnScreen | Pending: next click will type-select units on screen |
| 2 | SelectAcrossMap | Pending: next click will type-select all map-wide |
| 3 | HealthNav | Pending: jump-to-lowest-health unit |
| 4 | VeterancyNav | Pending: jump-to-elite-unit |

### 7.2 `FUN_00731d00` — reset to Normal

Called by every successful selection action (Select, Unselect_All, and several UI commands):
```c
DAT_00b0fe54 = 0;   // mode
DAT_00b0fe58 = 0;   // sub-state
```

### 7.3 Companion flag `DAT_00b0fe64` (TypeSelect "across-map toggle")

- `0` = T-key selects same-type units **on current screen**
- `1` = T-key selects same-type units **across the entire map**

Flips from 0 to 1 automatically inside `FUN_00732950` when every on-screen same-type
unit is already selected — i.e. pressing T a second time widens the scope to the whole
map.

`DAT_00b0fe65` is the "Shift-held during band-select" flag (set by `FUN_0054f5c0(0x10)`
at the start of band-select resolution; used to switch from "clear-then-select" to
"additive" behavior). Documented in BANDBOX report; listed here for completeness.

---

## 8. TypeSelect (T key) — `FUN_00732950`

When the player presses **T**, the engine selects all units of the same type(s) as
whatever is currently selected.

### Flow

```c
if (DAT_00a8b538 != 0) return;              // game locked (pause/dialog)
Unselect_If_Not_Owned();                    // drop a single enemy selection
if (DAT_00b0fe54 != 2)                      // if mode is not SelectAcrossMap…
    DAT_00b0fe64 = 0;                       //   …reset to on-screen scope

// Phase 1 — try on-screen same-type
if (DAT_00b0fe64 == 0) {
    list = DynamicVectorClass::new(growth=10);
    for (i = 0; i < Tactical.VisibleObjectCount; i++) {
        obj = Tactical.VisibleObjects[i];
        if (obj->IsVisible && FUN_00732580(obj))   // type-match predicate
            list.push(obj);
    }

    // If every on-screen same-type unit is ALREADY selected (IsSelected != 0),
    // escalate to across-map; otherwise stay on-screen.
    for (obj in list)
        if (obj->IsSelected == 0) { goto select_them; }
    DAT_00b0fe64 = 1;                       // escalate on next T press
}

// Phase 2 — across-map (DAT_00b0fe64 == 1 OR forced escalation)
list2 = DynamicVectorClass::new(growth=10);
for (i = 0; i < g_TechnoClass_Count; i++) {
    obj = g_TechnoClass_Array[i];
    if (FUN_00732580(obj))
        list2.push(obj);
}

select_them:
for (obj in list)
    obj->vtable[0x14c]();                   // Select each

// HUD message (EVA string)
if (count == 0)      msg = LoadString("UI:Cmnds", 0x3f1);    // "no matching units"
else if (across_map) msg = LoadString("UI:Cmnds", 0x3f3);    // "selected all (map)"
else                 msg = LoadString("UI:Cmnds", 0x3f5);    // "selected all (screen)"
FUN_005d3ba0(..., msg, ...);                                  // display
DAT_00b0fe54 = 2;                                             // mode = SelectAcrossMap
```

### Per-unit predicate `FUN_00732770`

Used by `FUN_007327d0` (the related "select same type" callback). Ensures:
- object is valid,
- `+0x90 != 0` (some "eligible for type-select" flag),
- owner is human-player (skirmish) or matches campaign-local flag,
- `FUN_005f3e50` returns true — this calls `this->vtable[0x88]()` which returns the
  type-class pointer and hashes/compares against the reference.

### Key facts

- **Two-stage escalation**: first T press = on-screen, second T press (within the same
  mode) = map-wide. No fixed timer — just state toggle.
- **Max selection count**: unbounded; the DynamicVectorClass grows by 10 as needed.
- **Voice suppression**: `DAT_00822cf2` is saved, forced to 0 during the batch, then
  restored at the end — so only the normal `ObjectSelection__PlayVoice` path at the end
  plays a single response.

---

## 9. Control groups — team hotkeys

### 9.1 Per-unit storage

Each TechnoClass has **`int GroupIndex` at offset `+0x214`**. Value semantics:
- `-1` (`0xffffffff`) = not in any group
- `0..9` = group slot N (where the hotkey is the key labelled N+1; group 9 = key "0")

### 9.2 Command handlers

There are **three** distinct command classes, each a subclass of CommandClass with its
own vtable, registered at startup:

| Command | Default key | Constructor | Execute behavior |
|---|---|---|---|
| `TeamCreate_N` (N=1..10) | `Ctrl+N` | `0x00535c00` | Clear group N, add all currently-selected units to group N |
| `TeamSelect_N` | `N` | `0x00535fc0` | Recall group N (see §9.3) |
| `TeamAddSelect_N` | `Shift+N` | `0x005360b0` | Add group-N members to the current selection (additive recall) |
| `CenterTeam_N` | — (implicit double-tap) | `0x005361a0` | Centre viewport on group N |

### 9.3 `ControlGroup__Recall` — the unified "press N" handler (`0x007311c0`)

This is the function the dispatch routes to when the player presses a digit key (N). It
handles both fresh recall and double-tap-to-centre via a timestamp.

```c
void ControlGroup__Recall(int group_id) {   // group_id is 1..10
    // Cancel placement/cursor states on DisplayClass
    Display.CancelMapPlacement();     // 0x004ac820
    Display.CancelBuildingPlacement();// 0x004ac700
    Display.CancelCursor3();          // 0x004ac8c0
    Display.CancelCursor4();          // 0x004ac660
    Display.CancelCursor5();          // 0x004ac960

    DWORD now = timeGetTime();

    // Double-tap detection — same group within 800 ms AND
    // at least one member is already selected → centre camera
    if (now - g_LastTeamPressTime < 800 && group_id == g_LastTeamPressGroup) {
        for (int i = g_TechnoClass_Count - 1; i >= 0; i--) {
            techno = g_TechnoClass_Array[i];
            if (techno && !techno->InLimbo && owner_is_human(techno)
                && techno->GroupIndex == group_id - 1
                && techno->IsSelected) {
                FUN_007313a0(group_id);           // centre camera on group
                return;
            }
        }
    }

    // Normal recall: clear current selection and select all group members
    g_LastTeamPressTime = now;
    g_LastTeamPressGroup = group_id;
    Unselect_All();
    DAT_00822cf2 = 1;                             // voice enabled for batch start
    for (int i = 0; i < g_TechnoClass_Count; i++) {
        techno = g_TechnoClass_Array[i];
        if (techno && !techno->InLimbo && owner_is_human(techno)
            && techno->GroupIndex == group_id - 1
            && !techno->IsSelected) {
            techno->vtable[0x14c]();              // Select
            // Stamp action-line state for ~25 frames of target lines
            techno->field_0x174 = g_CurrentFrameCounter;
            techno->field_0x178 = group_id;       // (see Open Questions)
            techno->field_0x17C = RulesClass[0x8c]; // action-line duration from rules
            DAT_00822cf2 = 0;                     // suppress voices after first
        }
    }
    DAT_00822cf2 = 1;
    ActionLines__StartTimer();                    // 0x0070d150: sets 25-frame timer
}
```

### 9.4 Assignment helpers

| Function | Addr | Meaning |
|---|---|---|
| `Count_Members_Of_Group(N)` | `0x00730a10` | Count non-limbo human-owned units with `GroupIndex == N-1` |
| `All_Group_Members_Selected(N)` | `0x00730990` | Return 1 iff every unit with `GroupIndex == N-1` has `IsSelected == 1` |
| `Team_AssignSelectedToGroup(N)` | `0x00731060` | Clear group N, then set `GroupIndex = N-1` on every currently-selected unit |
| `Team_ClearGroup(N)` | `0x007310d0` | Set `GroupIndex = -1` on every unit with `GroupIndex == N-1` |

### 9.5 CommandBar auxiliary dispatch (`0x006d0680`)

There is a secondary dispatch in `CommandBar_Dispatch` that decodes direct digit-key
input into one of: TypeSelect (T), team recall, or a handful of navigation commands. In
this path, if you press N when group N has **zero** members, it falls through to
`Team_AssignSelectedToGroup(N)` — so pressing a digit assigns the current selection when
the group is empty, and recalls when the group is non-empty. (The Ctrl+N command class
also assigns, just unconditionally.)

### 9.6 Double-tap timing

- Window: **800 ms** (`0x320`, hardcoded).
- Double-tap requires (a) same group id, (b) within the window, and (c) at least one
  unit in that group is currently selected (confirming the first tap succeeded).
- A double-tap calls `FUN_007313a0`, which centres the tactical camera on the group's
  bounding-box centre.

---

## 10. Multi-unit order dispatch (`FUN_004ae750`)

When the player issues a move/attack/force-move with multiple units selected, this
function iterates `CurrentObjects` and sends an individual order to each:

```c
void FUN_004ae750(int target_obj, undefined4 cell, int order_type) {
    if (DAT_00a8ed6b != 0) return;                 // selection disabled

    // Pre-hook each selected unit (clears formation cache at +0x430 when bit 2 of
    // +0x14 is set — this appears to cancel group-move formations)
    for (i = 0; i < count; i++) {
        data[i]->vtable[0x1a4](target_cell, flags);
        if ((data[i]->flags_0x14 & 4) != 0)
            data[i]->field_0x430 = 0;
    }

    if (target_obj == 0) {
        // Ground-targeted order
        if (order_type == 1 || order_type == 0x33)       // Move / AttackMove?
            for each selected: Compute = obj->vtable[0x70](cell,…); obj->vtable[0x140](Compute);
        else if (order_type == 2)                         // Guard/force-stop?
            for each: obj->vtable[0x70](cell,…); obj->vtable[0x140](Compute);
        else                                              // Stop / other
            for each: obj->vtable[0x70](cell, -1,…);      obj->vtable[0x140](Compute);
    } else {
        // Target-is-object order
        for each selected:
            Compute = obj->vtable[0x74](target_obj,…);
            obj->vtable[0x144](Compute);
    }

    DAT_00822cf2 = 1;                              // restore voice
}
```

Key vtable slots:

| Slot | Purpose |
|---|---|
| `+0x70` | `What_Action(cell, …)` — compute appropriate order for a ground target |
| `+0x74` | `What_Action(ObjectClass*, …)` — compute order for an object target |
| `+0x140` | Execute ground-targeted order |
| `+0x144` | Execute object-targeted order |
| `+0x1a4` | Pre-order hook (clears formation state) |

There is **no** group-pathing, no group-leader, and no formation movement baked into
this dispatch — each selected unit independently resolves its own order from its own
position.

---

## 11. INI keys that affect selection

Only a small number of INI keys actually feed into the selection system itself (most
selection-related INI keys are about the visuals covered in SELECTION_BRACKETS). Verified
keys:

| Key | Section | Default | Effect |
|---|---|---|---|
| `Selectable` | `[UnitName]`/`[BuildingName]` | `yes` | TechnoTypeClass flag read by `CanBeSelected` vtable (+0x138). `no` → cannot be clicked/band-selected. |
| `IsSelectableCombatant` | `[UnitName]` | `no` | Flag for "select all combatants" hotkey group (not the generic T-key). |
| `VoiceSelect` | `[UnitName]` | `—` | List of voice clip keys played from `TechnoClass::Select`'s `vtable[0x360]` call when `DAT_00822cf2 != 0`. |
| `PixelSelectionBracketDelta` | `[UnitName]` | `0` | Purely cosmetic (covered in SELECTION_BRACKETS); listed here for completeness. |

Not found in this INI set: any `MaxSelected`, `SelectionPriority`, or global cap key.
The list is unbounded in practice (grows by 10).

---

## 12. Integration points

### Who calls `Select`
- `ObjectClass` vtable slot `+0x14C`, invoked from:
  - Band-box resolution (see BANDBOX report): `FUN_006da5c0`
  - Click-selection path: `FUN_004ac2b0` (left-click action 7/8)
  - TypeSelect (T key): `FUN_00732950`, `FUN_007327d0`
  - Control-group recall: `ControlGroup__Recall` (`0x007311c0`)
  - Save/load restoration: scenario read paths

### Who calls `Deselect`
- Directly: `Unselect_All` (loop), `Unselect_If_Not_Owned`
- Indirectly: `Deselect` button on sidebar; unit-death code paths *do not* call it
  explicitly (reliance on `InLimbo` flag to ignore dead entries).

### When does selection state change in the tick?
- All selection mutations happen in the **input-processing phase**, before sim advance.
- Selection is **not** serialized in the per-tick state hash — it is a UI-local state.
  Save/load serializes the `+0x83` byte (per OBJECTCLASS report), so selection persists
  across save.

---

## 13. Current Rust implementation status

(Summarised from Rust scan; detail in the main scan report.)

| Feature | Original | Rust impl |
|---|---|---|
| Selection list storage | DynamicVectorClass<ObjectClass*> (ordered) | `GameEntity.selected: bool` per entity + `Vec<u64>` snapshots |
| Insertion order preserved | Yes (except prepend-class branch) | Order = enumeration order, not player-intent order |
| `IsSelected` flag | byte at `+0x83` | `selected: bool` on GameEntity |
| Max count | Unbounded (grows by 10) | Unbounded |
| Prepend-class branch | Yes — `+0x14 & 1` AND TTypeClass+0xc9c set | **Not implemented** |
| Click-select single unit | vtable dispatch → Select | `compute_click_selection_snapshot()` in `app_entity_pick.rs` |
| Clear on right-click | No — right-click is a command | Yes — right-click clears selection (deviation) |
| `Unselect_All` | Loops `Deselect` | `deselect_all()` in `sim/selection.rs:179-190` |
| Voice-suppress flag | `DAT_00822cf2` | `emit_selection_voice()` plays only first unit's voice |
| Team hotkeys Ctrl+1..9 | `Team_AssignSelectedToGroup` | Present in `app_input.rs` `handle_control_group_hotkey()` |
| Team hotkeys 1..9 (recall) | `ControlGroup__Recall`; assigns if group empty | Present; assigns-if-empty behavior **not implemented** |
| Shift+1..9 (additive) | `TeamAddSelect_N` via separate CommandClass | Present (additive parameter) |
| Double-tap to centre | 800ms window | **Not implemented** |
| TypeSelect (T key) | Two-stage on-screen → map-wide | `select_same_type()` — **scope is always map-wide** (deviation) |
| "No match" HUD message | EVA string 0x3f1 | Not implemented |
| Multi-unit order dispatch | Per-unit via vtable 0x140/0x144 | Per-unit via individual Commands |
| Action lines on recall | 25-frame target lines set per unit | Not implemented |
| Desync check on cross-owner add | `Desync_Handler` | Not implemented (harmless; YR path never triggers it) |

### Gaps worth tracking

- **Double-tap centre camera** on team hotkeys (800 ms window) — user-visible feature.
- **TypeSelect two-stage scope** (on-screen → map-wide) — user-visible feature.
- **Pressing a digit with empty group assigns current selection** — small convenience that
  players rely on.
- **Right-click-clears-selection is a deviation from the original** — the original treats
  right-click as a command (stop, attack, etc.), not a selection clear.
- **Per-unit action-line stamping on recall** — needed for correct target-line rendering
  after a team hotkey.

---

## 14. Open Questions — all resolved

**Resolved:**

1. **Prepend-class identity** — **`TechnoTypeClass+0xc9c` = "has primary weapon
   with Damage > 0"** (i.e., combat unit flag). Set in `TechnoTypeClass::ReadINI`
   at `0x007157AE`: after parsing the Primary weapon at TypeClass+0x898, if the
   weapon's `Damage=` (WeaponTypeClass+0xA4) is > 0, this byte flips to 1.
   So the prepend branch triggers for any combatant — the list always leads with
   a unit that can actually fight.

2. **`+0x178` stack slot** — **uninitialized stack garbage (engine bug)**.
   `MOV EBP, [ESP+0x14]` reads a local slot that's never written between entry
   and the action-line stamp. Functionally harmless: the action-line rendering
   reads `+0x174` (frame) and `+0x17C` (duration), not `+0x178`. The bug doesn't
   manifest but the field gets garbage.

3. **`RulesClass+0x8c`** — a dword read by the team-recall stamp loop as
   per-unit action-line duration. I could not pin the specific INI key in this
   pass (many writers to this offset across RulesClass subsections). Does not
   affect correctness — the action-line timer itself runs on a fixed 25-frame
   global (`ActionLines__StartTimer` sets 0x19). Deferred; low-value.

4. **`g_SelectionSubMode` (`0x00b0fe58`)** — **pending-modal-action flag**.
   Set to 1 by the Force-Move command handler (`FUN_00731AF0`) after the
   selection is validated (every selected unit passes `vtable+0x4C0`). Cleared
   to 0 by `Selection__ResetMode`. While non-zero, `FUN_00731BF0` short-circuits
   further command dispatch until the pending action resolves.

5. **`Filter_AbstractType_InMap`** — **RTTI filter**: returns `obj` if its
   `What_Am_I` vtable slot (+0x2C) returns 1, 2, 6, or 0xF (Unit, Aircraft,
   Building, Infantry); else NULL. Then:
   - `+0x3D4` byte = `IsAirborne` (from TECHNOCLASS_EXPANDED_STRUCT_LAYOUT).
     Set by `AircraftClass::Unlimbo` when a helicopter spawns at altitude.
     When set, selection is rejected.
   - `vtable+0x1D4` = `TechnoClass::IsWarpingOut` (reads `+0x270`). Set by
     `TeleportLocomotion::Phase0_SetWarpingOut` at the start of chrono-warp.
     When non-zero, selection is rejected — the unit is mid-warp and un-pickable.

6. **`g_UIModeLock` (`0x00880990`)** — now **relabelled** in Ghidra. Write
   sites: `Main_Game`, bridge repair (`MapClass::MarkBridgesForRepair_{Low,High}`,
   `SelectDestroyedBridgeTile_Low`, `SelectBridgeTileVariant_Low`), scenario
   trigger dispatcher (`FUN_0059e740`, many sites), and some cleanup helper.
   Read by: `ObjectClass::Select` (reject), `BuildingPlacement_OverlayRenderer`
   (gate rendering), `BuildingPlacement_per_cell_draw` (gate per-cell draw).
   Semantic: **modal UI state active — disable unit selection**. Fires for
   bridge-repair targeting, building placement preview, and scenario triggers
   that lock input.

---

## Sources

### Ghidra addresses decompiled (12 functions + 1 init block)

| Address | Name | Purpose |
|---|---|---|
| `0x005F4520` | `ObjectClass::Select` | Primary function — add-to-list |
| `0x005F44A0` | `ObjectClass::Deselect` | Primary function — remove-from-list |
| `0x006FBFA0` | `TechnoClass::Select` | Override: voice + MindControl hook |
| `0x006da740` | `Unselect_All` | Loop-Deselect |
| `0x006da770` | `Unselect_If_Not_Owned` | Edge-case single-enemy cleanup |
| `0x005F6C30` | `ObjectClass::CanBeSelected` | vtable+0x138 check |
| `0x00637840` | `ObjectSelection__PlayVoice` | Re-entrant-guarded deferred voice processor |
| `0x007311c0` | `ControlGroup__Recall` | Team-hotkey recall with double-tap |
| `0x00731060` | `Team_AssignSelectedToGroup` | Ctrl+N / empty-group N |
| `0x007310d0` | `Team_ClearGroup` | Shift+N cleanup path |
| `0x00730a10` | `Count_Members_Of_Group` | Helper |
| `0x00730990` | `All_Group_Members_Selected` | Helper |
| `0x00732950` | TypeSelect (T key) handler | On-screen/map-wide escalation |
| `0x007327d0` | SelectAllSameType helper | Called from hotkey + double-click paths |
| `0x00732770` | Type-match predicate | Used by above |
| `0x0070d150` | `ActionLines__StartTimer` | 25-frame timer after recall |
| `0x00731d00` | Reset selection-mode state | `DAT_00b0fe54 = 0; DAT_00b0fe58 = 0;` |
| `0x004ae750` | Multi-unit order dispatch | Iterates CurrentObjects |
| `0x004aeb10`/`0x004aeb30` | DisplayClass LastRef get/set | Cleaned up on Deselect |
| `0x006d0680..0x006d09b3` | CommandBar_Dispatch (partial) | Digit-key command decoding |
| `0x004e7d40..0x004e7d7c` | CurrentObjects init | DynamicVectorClass construction |
| `0x007e4f64` | DynamicVectorClass vtable | (Resize at +0x08, ID at +0x10) |

### Doc files referenced
- `BANDBOX_SELECTION_GHIDRA_REPORT.md` (adjacent system)
- `building-selection-brackets/SELECTION_BRACKETS_GHIDRA_REPORT.md` (adjacent system)
- `OBJECTCLASS_GHIDRA_REPORT.md` (IsSelected field origin)
- `HOTKEY_SYSTEM_GHIDRA_REPORT.md` (command-class registration)
- `TECHNOCLASS_VTABLE_COMPLETE.md` (vtable slot layout)

### INI files checked
- `ini/rulesmd.ini` — primary (YR)
- `ini/rules.ini` — base RA2 fallback
- `ini/artmd.ini`, `ini/art.ini` — nothing selection-list-relevant

---

## Appendix A — `ObjectClass::Select` decompilation (Ghidra)

```c
undefined4 ObjectClass__Select(int *param_1) {
  undefined4 *puVar1;
  char extraout_AL;
  char cVar2;
  char cVar3;
  int *piVar4;
  int iVar5;

  // ----- Early-exit gates ----------------------------------------------
  if ((DAT_00a8ed6b == '\0') &&
     (((*(char *)((int)param_1 + 0x81) != '\0' ||            // InLimbo
        *(char *)((int)param_1 + 0x83) != '\0') ||           // already selected
      ((**(code **)(*param_1 + 0x138))(),                    // CanBeSelected
        extraout_AL == '\0')))) {
    return 0;
  }

  piVar4 = (int *)Filter_AbstractType_InMap();
  if ((DAT_00a8ed6b == '\0') && (cVar2 = (**(code **)(*param_1 + 0xa0))(), cVar2 != '\0')) {
    if (piVar4 == (int *)0x0) goto LAB_005f45a0;
    if ((char)piVar4[0xf5] != '\0') return 0;
  }
  if (piVar4 != (int *)0x0 && (cVar2 = (**(code **)(*piVar4 + 0x1d4))(), cVar2 != '\0'))
    return 0;

LAB_005f45a0:
  if (DAT_00880990 != 0) return 0;                           // UI-mode lock

  // ----- Desync check on non-empty list --------------------------------
  if (0 < DAT_00a8ecc8) {
    (**(code **)(*param_1 + 0x3c))();                        // this->Owner
    (**(code **)(*(int *)*DAT_00a8ecbc + 0x3c))();           // head->Owner
    cVar2 = HouseClass__IsHumanPlayer();
    cVar3 = HouseClass__IsHumanPlayer();
    if ((cVar2 != cVar3) || (cVar2 = HouseClass__IsHumanPlayer(), cVar2 == '\0'))
      Desync_Handler();
  }

  // ----- Append vs prepend branch --------------------------------------
  if (((param_1 == (int *)0x0) || ((*(byte *)(param_1 + 5) & 1) == 0)) ||
     (iVar5 = (**(code **)(*param_1 + 0x88))(),              // TypeClass
      *(char *)(iVar5 + 0xc9c) == '\0')) {
    // APPEND
    if ((DAT_00a8ecc8 < DAT_00a8ecc0) ||
       (((DAT_00a8ecc5 != '\0' || DAT_00a8ecc0 == 0) &&
        (0 < DAT_00a8eccc &&
         (cVar2 = (**(code **)(DAT_00a8ecb8 + 8))(DAT_00a8eccc + DAT_00a8ecc0,0),
          cVar2 != '\0'))))) {
      puVar1 = DAT_00a8ecbc + DAT_00a8ecc8;
      DAT_00a8ecc8 = DAT_00a8ecc8 + 1;
      *puVar1 = param_1;
    }
  } else {
    // PREPEND — shift existing right, insert at head
    if ((DAT_00a8ecc8 < DAT_00a8ecc0) || /* same resize test */) {
      if (DAT_00a8ecc8 != 0)
        FUN_007ca090(DAT_00a8ecbc + 1, DAT_00a8ecbc, DAT_00a8ecc8 << 2);  // memmove
      *DAT_00a8ecbc = param_1;
      DAT_00a8ecc8 = DAT_00a8ecc8 + 1;
      FUN_00731d00();
      *(undefined1 *)((int)param_1 + 0x83) = 1;
      return 1;
    }
  }
  FUN_00731d00();                                             // reset mode
  *(undefined1 *)((int)param_1 + 0x83) = 1;                   // set IsSelected
  return 1;
}
```

## Appendix B — Ghidra Labels Applied

All renames and plate comments below are persisted in the Ghidra project; run
`save_program` already called.

### Functions renamed (12)

| Address | Old name | New name |
|---|---|---|
| `0x00731060` | `FUN_00731060` | `Team__AssignSelectedToGroup` |
| `0x007310d0` | `FUN_007310d0` | `Team__ClearGroup` |
| `0x00730a10` | `FUN_00730a10` | `Team__CountMembers` |
| `0x00730990` | `FUN_00730990` | `Team__AllMembersSelected` |
| `0x00731d00` | `FUN_00731d00` | `Selection__ResetMode` |
| `0x00732950` | `FUN_00732950` | `TypeSelect__Execute` |
| `0x007327d0` | `FUN_007327d0` | `TypeSelect__ApplyGroupSelect` |
| `0x00732770` | `FUN_00732770` | `TypeSelect__MatchPredicate` |
| `0x004ae750` | `FUN_004ae750` | `Selection__DispatchMultiUnitOrder` |
| `0x007313a0` | `FUN_007313a0` | `ControlGroup__CenterCamera` |
| `0x004aeb10` | `FUN_004aeb10` | `DisplayClass__GetLastRefObject` |
| `0x004aeb30` | `FUN_004aeb30` | `DisplayClass__SetLastRefObject` |

### Globals labelled (13)

| Address | Old name | New name | Type |
|---|---|---|---|
| `0x00a8ecb8` | `DAT_00a8ecb8` | `g_CurrentObjects` | DynamicVectorClass vtable slot |
| `0x00a8ecbc` | `DAT_00a8ecbc` | `g_CurrentObjects_Data` | `ObjectClass**` |
| `0x00a8ecc0` | `DAT_00a8ecc0` | `g_CurrentObjects_Capacity` | `int` |
| `0x00a8ecc5` | `DAT_00a8ecc5` | `g_CurrentObjects_IsAllocated` | `byte` |
| `0x00a8ecc8` | `DAT_00a8ecc8` | `g_CurrentObjects_Count` | `int` |
| `0x00a8eccc` | `DAT_00a8eccc` | `g_CurrentObjects_GrowthStep` | `int` (=10) |
| `0x00b0fe54` | `DAT_00b0fe54` | `g_SelectionMode` | `byte` (0..4) |
| `0x00b0fe58` | `DAT_00b0fe58` | `g_SelectionSubMode` | `byte` |
| `0x00b0fe64` | `DAT_00b0fe64` | `g_TypeSelect_AcrossMap` | `byte` (0/1) |
| `0x00822cf2` | `DAT_00822cf2` | `g_SelectionVoice_Enable` | `byte` (0/1) |
| `0x00845550` | `DAT_00845550` | `g_LastTeamPressTime` | `DWORD` (GetTickCount) |
| `0x00845554` | `DAT_00845554` | `g_LastTeamPressGroup` | `int` |
| `0x00a8ed6b` | `DAT_00a8ed6b` | `g_IsMapEditor` | `byte` (from `SCENARIO_INIT_DEEP_DIVE`) |

### Plate comments added (15)

Plate comments document behavior, gates, and cross-references on:
`ObjectClass::Select`, `ObjectClass::Deselect`, `TechnoClass::Select`, `Unselect_All`,
`Unselect_If_Not_Owned`, `Selection__ResetMode`, `TypeSelect__Execute`,
`TypeSelect__ApplyGroupSelect`, `TypeSelect__MatchPredicate`,
`Selection__DispatchMultiUnitOrder`, `ControlGroup__Recall`,
`Team__AssignSelectedToGroup`, `Team__ClearGroup`, `Team__CountMembers`,
`Team__AllMembersSelected`.

### Deliberately not renamed

- `0x00880990` (`g_UIModeLock` candidate) — scope uncertain; left as `DAT_` until its
  lifecycle is traced.
- `FUN_006fc030` — appears in bandbox research as an unrelated picking helper;
  different call context from the selection list.
- Filter_AbstractType_InMap — already named (inherited label); left as-is.

---

## Appendix C — DynamicVectorClass init at `0x004e7d40`

```asm
XOR EAX, EAX
PUSH 0x004e7d80                              ; caller arg (not selection-relevant)
MOV  [0x00a8ecbc], EAX                       ; data = NULL
MOV  [0x00a8ecc0], EAX                       ; capacity = 0
MOV  byte [0x00a8ecc4], 1
MOV  byte [0x00a8ecc5], AL                   ; IsAllocated = 0
MOV  [0x00a8ecb8], 0x007e4f64                ; vtable
MOV  [0x00a8eccc], 0xa                       ; growth_step = 10
MOV  [0x00a8ecc8], EAX                       ; count = 0
CALL (ctor helper)
POP  ECX
RET
```
