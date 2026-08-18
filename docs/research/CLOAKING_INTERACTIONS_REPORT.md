# Cloaking State Interactions: Transports, Chronoshift, Mind Control, Disguise

Ghidra research report — cloaking state interactions with other game systems.
Confidence: HIGH for transport/chronoshift/mind-control paths, MEDIUM for disguise
(confirmed via vtable dispatch, field offsets, and decompilation of key functions).

## Key TechnoClass Fields Referenced

| Offset | Size  | Field               | Notes |
|--------|-------|---------------------|-------|
| +0x220 | DWORD | CloakState          | 0=Uncloaked, 1=Cloaking, 2=Cloaked, 3=Uncloaking |
| +0x224 | DWORD | CloakProgress       | Animation counter |
| +0x270 | BYTE  | IsWarpingIn         | Set by TeleportLocomotionClass on chrono warp arrival |
| +0x271 | BYTE  | IsWarpingOut        | Set by TeleportLocomotionClass during chrono warp departure |
| +0x2B4 | DWORD | MindControllerPtr   | Pointer to the TechnoClass that mind-controls this unit (0 = not controlled) |
| +0x3D2 | BYTE  | HasStealthAbility   | Runtime cloakable flag |
| +0x3D5 | BYTE  | (unknown flag)      | Checked alongside HasStealthAbility in Unlimbo |

TechnoTypeClass fields:
| Offset  | Field                    | Notes |
|---------|--------------------------|-------|
| +0x2A2  | VeteranAbilities[CLOAK]  | BYTE — veteran promotion grants cloaking |
| +0x2B4  | EliteAbilities[CLOAK]    | BYTE — elite promotion grants cloaking |
| +0x310  | CloakingSpeed            | DWORD — frames between cloak steps |
| +0xCD0  | Cloakable                | BYTE — INI `Cloakable=` flag |
| +0xCD5  | (unknown)                | Checked in CloakingTick state 0 alongside CD0 |
| +0xD2F  | CanDisguise              | BYTE — INI `CanDisguise=` |
| +0xD30  | PermaDisguise            | BYTE — INI `PermaDisguise=` |
| +0xD31  | DisguiseWhenStill        | BYTE — INI `DisguiseWhenStill=` |

---

## 1. Transport Enter / Exit

### Entering a Transport (Limbo)

**Function:** `FUN_004D9720` (TechnoClass/FootClass vtable+0xDC = Limbo override)
**Address:** 0x004D9720, calls base `ObjectClass::Limbo` at 0x005F5280

When a unit enters a transport, `Limbo(false)` is called on the entering unit.
**CloakState is NOT explicitly reset by Limbo.** The Limbo function handles:
- Radar tracking removal (vtable+0x274 with arg 3)
- Removing the unit from the map (ObjectClass::Limbo at 0x5F5280)
- Shroud/visibility removal (vtable+0x150)

The CloakState field at +0x220 remains unchanged through Limbo — whatever state
the unit was in (Uncloaked, Cloaking, Cloaked, Uncloaking) persists in memory while
in limbo. However, since the unit is off-map, CloakingTick (0x6FB740) effectively
does nothing because the unit has no cell position.

### Exiting a Transport (Unlimbo)

**Function:** `UnitClass::Unlimbo` at 0x00737BA0
**Address:** 0x737BA0, calls base `FUN_004D7170` (FootClass::Unlimbo)

**Critical behavior at 0x737BEB:**
```c
if (*(char*)(this + 0x3D2) != '\0' && *(char*)(this + 0x3D5) == '\0') {
    *(int*)(this + 0x220) = 2;  // CloakState = CLOAKED (instantly!)
}
```

When a cloakable unit (HasStealthAbility at +0x3D2) exits a transport, its CloakState
is set **directly to 2 (Cloaked)** — skipping the Cloaking animation entirely. The unit
appears immediately invisible on the map. The second check (+0x3D5 == false) appears
to be an override flag that can prevent this instant re-cloak.

**Summary:** Cloakable units that enter a transport retain their CloakState in memory.
On exit, they are force-cloaked to state 2 (fully cloaked) instantly, bypassing the
normal state 0->1->2 transition animation.

---

## 2. Chronoshift / Teleport Warping

### IsWarping Virtual Functions

**vtable+0x1D4** (UnitClass: 0x0070C5B0): Returns `byte [this + 0x270]` = **IsWarpingIn**
**vtable+0x1D8** (UnitClass: 0x0070C5C0): Returns `byte [this + 0x271]` = **IsWarpingOut**

Both fields are initialized to 0 in `TechnoClass::Constructor` (0x6F2B40).

### CloakingTick Warp Guard (0x6FB783-0x6FB797)

In `CloakingTick` (0x6FB740), when CloakState == 0 (Uncloaked), the function checks
whether the unit should begin auto-cloaking. One of the guard conditions is:

```asm
006fb783: CALL dword ptr [EDX + 0x1d4]   ; IsWarpingIn()
006fb789: TEST AL,AL
006fb78b: JNZ  short skip_cloak          ; if warping in, don't auto-cloak
006fb78d: MOV  EAX,dword ptr [ESI]
006fb78f: MOV  ECX,ESI
006fb791: CALL dword ptr [EAX + 0x1d8]   ; IsWarpingOut()
006fb797: TEST AL,AL
006fb799: JZ   proceed_to_cloak          ; if NOT warping, allow cloak
```

**Effect:** A unit that is currently warping (chrono-in or chrono-out) is prevented
from entering the cloaking state machine. The unit stays at CloakState 0 while warping.
Auto-cloaking is deferred until both warp flags are cleared.

### TeleportLocomotionClass Warp Sequence

**Warp departure** (function containing 0x719579):
```c
*(byte*)(unit + 0x271) = 1;  // Set IsWarpingOut
```
Then checks if UnitClass with `ChronoTriggered` type flag — if so, immediately clears
the flag (unit doesn't visually warp). Otherwise, sets a timer based on distance and
`Rules->ChronoDistanceFactor` / `Rules->ChronoMinimumDelay` (offsets 0xBF4-0xC00).

After the timer expires, calls `vtable+0x124` (ProcessCloakMode) and `vtable+0x1CC`
to finalize departure.

**Warp arrival** (function `FUN_0071AF20` at 0x71AF20):
```c
*(byte*)(unit + 0x270) = 1;   // Set IsWarpingIn
vtable+0x124(2);               // ProcessCloakMode(2) — force cloak visual
```
If unit type has `ChronoTriggered` flag, also calls `BuildingClass__StartCloaking` on the
destination building. Then calls `vtable+0x124(2)` again and updates visibility.

**Warp completion** (function `FUN_00719790` at 0x719790):
```c
*(int*)(unit + 0x280) = 0;     // Clear warp state
// IsWarpingIn (0x270) is cleared elsewhere in the sequence
```

### Visual Shimmer (0x04 flag)

In `TechnoClass::Draw` (0x706640), after computing the base draw flags:
```c
if (IsWarpingIn() || IsWarpingOut()) {
    if (RTTI != UnitClass || !unit->field_0x6D3) {
        drawFlags |= 0x2004;  // chrono shimmer
    }
}
```

The `0x04` bit (combined with `0x2000` base) triggers the chrono warp visual effect
in the blitter. This creates the characteristic blue shimmer seen when a Chrono Legionnaire
or Chronosphere warps a unit. The `ModifyCloakDrawFlags` function (0x70ED80) cycles the
shimmer through frame-based animation phases.

---

## 3. Mind Control Scatter on Cloak

### CloakingTick State 1->2 Transition (0x6FBA98-0x6FBC0B)

When a unit transitions from CloakState 1 (Cloaking) to CloakState 2 (Cloaked), the
CloakingTick function performs a mind control sweep:

**Step 1:** Set CloakState = 2 at 0x6FBA98
```asm
006fba98: MOV dword ptr [ESI + 0x220], 0x2   ; CloakState = CLOAKED
```

**Step 2:** Call ProcessCloakMode(2) at 0x6FBACD
```asm
006fbacd: CALL dword ptr [EDX + 0x124]       ; ProcessCloakMode(2)
```

**Step 3:** UnitClass MCV check (0x6FBAD7-0x6FBAF2)
```asm
006fbad7: CALL dword ptr [EAX + 0x2c]        ; GetRTTI()
006fbada: CMP  EAX, 0x1                       ; == UnitClass?
006fbadf: CMP  dword ptr [ESI + 0x6cc], -1   ; DeployTarget != -1?
006fbae8: CALL dword ptr [EDX + 0xfc]         ; StartUncloaking(false)
```
If the cloaking unit is a UnitClass with a valid deploy target (+0x6CC != -1), it
force-uncloaks. This prevents cloaked MCV deploy scenarios.

**Step 4:** Mind control sweep (0x6FBAF7-0x6FBBBC)

Iterates `g_TechnoClass_Array` (global at 0x00A8EC7C, count at 0x00A8EC88):
```asm
006fbb35: CMP dword ptr [EDI + 0x2b4], ESI   ; unit->MindControllerPtr == this?
```

For each unit whose `MindControllerPtr` (+0x2B4) equals the cloaking unit (ESI),
the code performs visibility checks:
- Gets the mind-controlled unit's owner house (+0x21C -> +0x30 = house index)
- Calls `CellClass::GapCountForHouse` to check if the unit is in gap generator fog
- If visible (in gap fog or same house as controller), adds to collection array
- Collection uses a `DynamicVectorClass` with initial capacity 10 (vtable 0x7E17AC)

**Step 5:** Limbo self and release mind-controlled units (0x6FBBC3-0x6FBC0B)
```asm
006fbbc8: CALL dword ptr [EDX + 0xdc]        ; self->Limbo(false)
006fbbe3: CALL dword ptr [EDX + 0x3c8]       ; victim->ScatterFromMindControl(self)
```

First, the cloaking unit Limbos itself (removes from map).
Then for each collected mind-controlled unit, calls vtable+0x3C8 which is the
**ScatterFromMindControl** handler.

### ScatterFromMindControl (vtable+0x3C8)

**InfantryClass version:** 0x0051B1F0
**UnitClass version:** 0x006FDA00

The InfantryClass version (clearest decompilation):
```c
void InfantryClass::ScatterFromMindControl(TechnoClass* source) {
    if (source == this->MindControllerPtr) {
        goto scatter_only;  // Controller cloaked — just scatter, keep MC link
    }
    // If source != controller: cancel mission, clear tether
    // ...
scatter_only:
    FUN_006fcdb0(source);  // MindControl assignment function
    // Check if unit should scatter to new position
    if (this->NavComQueue == 0 && this->TypeClass->GuardRange != 0) {
        this->vfunc_0x480(source, 1);  // Scatter to random position
    }
}
```

**`FUN_006fcdb0` (MindControl assignment)** at 0x6FCDB0:
```c
void TechnoClass::SetMindController(TechnoClass* source) {
    this->field_0x50C = 0;  // Clear some state
    if (source == this->MindControllerPtr) {
        return;  // Source IS the current controller — NO CHANGE to MC link
    }
    // ... complex logic for assigning new controller
    this->MindControllerPtr = new_controller;  // May be NULL to release
}
```

**Key finding:** When `source == MindControllerPtr` (which is always the case during
cloaking scatter), the function returns early WITHOUT changing the mind control link.
The mind control relationship PERSISTS when the controller cloaks. The mind-controlled
units are only SCATTERED to adjacent cells — they are NOT released from mind control.

### DoUncloak (vtable+0x420) Mind Control Handling

**Function:** 0x006F4EB0

The same mind control sweep pattern appears in this function (0x6F4F9B-0x6F5032).
When a cloaked unit with mind-controlled subjects becomes visible/uncloaks:
1. Collects all units where `[unit + 0x2B4] == this`
2. Calls `vtable+0x460(0)` (StartCloaking) on self
3. Calls `vtable+0x3C8(this)` on each mind-controlled unit (scatter)

This ensures mind-controlled units get positional updates when their controller's
visibility changes.

---

## 4. Disguise System Independence

### Separate Systems

**Disguise and cloaking are completely independent systems.** They use different fields,
different vtable entries, and different state machines.

**Disguise fields** (TechnoTypeClass):
- +0xD2F: `CanDisguise` (INI key "CanDisguise")
- +0xD30: `PermaDisguise` (INI key "PermaDisguise")
- +0xD31: `DisguiseWhenStill` (INI key "DisguiseWhenStill")

**Disguise instance field** (TechnoClass):
- +0x1D8: BYTE — appears to be a disguise-active flag (returned by vtable+0xC4)
  - NOT to be confused with vtable+0x1D8 which is IsWarpingOut (different vtable function)

### No Cross-References in CloakingTick

The `CloakingTick` function (0x6FB740) **never reads or writes** any disguise field.
The disguise type fields (+0xD2F, +0xD30, +0xD31) are only referenced from:
- `TechnoTypeClass::ReadINI` (INI parsing)
- Various draw/render functions

The `CloakingTick` only checks:
1. CloakState (+0x220)
2. CloakProgress (+0x224)
3. CloakStepTimer (+0x22C)
4. CloakingSpeed (+0x238)
5. IsWarpingIn/Out (vtable+0x1D4/0x1D8 -> +0x270/+0x271)
6. IsCrashing (vtable+0x37C)
7. IsFiring (vtable+0x380)
8. CanAutoCloak (vtable+0x2A0/0x288)
9. ShouldUncloak (vtable+0x2A4)
10. MindControllerPtr (+0x2B4) — only during state 1->2 transition

### Draw-Time Separation

In `TechnoClass::Draw` (0x706640):
- **Cloak visual** is applied via `GetVisualState` (vtable+0x68) which returns states 0-5
  based on CloakState and CloakProgress
- **Warp visual** (0x2004 flag) is applied by checking IsWarpingIn/IsWarpingOut
- **Disguise visual** appears to be handled by vtable+0xC4 -> vtable+0x43C
  (`ModifyCloakDrawFlags`) which is a completely separate code path

A unit CAN be simultaneously cloaked AND disguised (e.g., Mirage Tank has both
`Cloakable=yes` and `DisguiseWhenStill=yes`). The systems layer independently:
disguise determines what TYPE the unit appears as to enemies, while cloaking determines
whether the unit is VISIBLE at all.

---

## VTable Function Reference

All offsets from the primary vtable (vtable[0]):

| Offset | Function | Notes |
|--------|----------|-------|
| +0x2C  | GetRTTI() | Returns type ID (1=Unit, 2=Aircraft, 5=Infantry, 6=Building) |
| +0x48  | GetCoords() | |
| +0x68  | GetVisualState(showToEnemy, house) | Returns 0-5 visual state for rendering |
| +0x84  | GetTechnoType() | Returns TechnoTypeClass pointer |
| +0xC4  | IsDisguised() | Returns byte at +0x1D8 |
| +0xDC  | Limbo(bool) | Remove from map |
| +0xFC  | StartUncloaking_Wrapper() | Calls vtable+0x45C(0) |
| +0x124 | ProcessCloakMode(int) | 0=remove cloak, 1=enable cloak, 2=force cloak |
| +0x150 | UpdateVisibility() | Shroud/radar update |
| +0x1B8 | GetCenterCoord() | |
| +0x1D4 | IsWarpingIn() | Returns byte at +0x270 |
| +0x1D8 | IsWarpingOut() | Returns byte at +0x271 |
| +0x274 | UpdateRadarTracking(int) | |
| +0x288 | CanAutoCloak_Internal() | Checks type flags, locomotor state |
| +0x2A0 | CanAutoCloak() | Full check including timers |
| +0x2A4 | ShouldUncloak() | |
| +0x37C | IsCrashing() | Checks EMP state |
| +0x380 | IsFiring() | Checks rearm timer |
| +0x3C8 | ScatterFromMindControl(source) | Scatter and optionally release mind control |
| +0x420 | OnUncloakedVisibleCheck() | Mind control sweep on visibility change |
| +0x43C | ModifyCloakDrawFlags(flags) | Add cloak/warp shimmer bits |
| +0x45C | StartUncloaking(bool playSound) | CloakState -> 3 |
| +0x460 | StartCloaking(bool playSound) | CloakState -> 1 |
| +0x480 | Scatter(target, forced) | Move to random adjacent cell |

---

## Implementation Notes for Rust Engine

1. **Transport enter:** Do NOT modify CloakState when a unit enters a transport.
   Keep it as-is in the EntityStore.

2. **Transport exit (Unlimbo):** If the unit has `HasStealthAbility` (+0x3D2) and
   NOT the override flag (+0x3D5), set CloakState directly to 2 (Cloaked). Skip
   the animation transition.

3. **Chronoshift:** Set IsWarpingIn/IsWarpingOut flags during teleport sequences.
   CloakingTick will automatically prevent auto-cloaking while either flag is set.
   Clear flags when warp animation completes.

4. **Mind control on cloak:** When transitioning state 1->2 (fully cloaked), iterate
   all entities checking `MindControllerPtr == self`. Scatter those units but do NOT
   release the mind control link. The scatter is positional only.

5. **Disguise:** Implement as a completely separate system. It shares no state with
   cloaking. A unit can be both cloaked and disguised simultaneously. The disguise
   affects WHAT the unit looks like; cloaking affects WHETHER it's visible.
