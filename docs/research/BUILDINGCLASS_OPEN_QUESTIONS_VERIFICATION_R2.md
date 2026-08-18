---
name: BuildingClass Round-2 Follow-up Verification
description: Binary verification of 5 follow-up questions from Round 1 plus structural survey of ExitObject dispatch (captured flag, HasSpotlight INI key, gap-gen state offset, vtable slots 279/280, SoundEvent fields, ExitObject top-level dispatch)
type: reference
---

# BuildingClass Round-2 Verification — Follow-ups + ExitObject Survey

**Date:** 2026-04-19
**Binary:** gamemd.exe
**Confidence:** HIGH — all follow-up findings verified from binary decompilation; ExitObject survey covers top-level dispatch only
**Active in YR:** Yes — all findings apply to standard YR skirmishes

Follow-up to `BUILDINGCLASS_OPEN_QUESTIONS_VERIFICATION.md`. This round
tightens 5 specific findings from the first pass and does a structural survey
of `ExitObject` (0x00443C60, the 6724-byte production exit dispatcher).

---

## 1. +0x6E3 is the "OwnershipChanged" flag — NOT bio-reactor

My Round-1 report speculated this might be a bio-reactor flag. **Verified in
this round — it's not.** It's the captured-building flag.

**Evidence:**

- **Constructor** `0x0043B740` at `0x0043B96C`: initializes to 0
  ```
  0043b96c: MOV byte ptr [ESI + 0x6e3], BL    ; BL = 0 (ctor zero-init block)
  ```
- **`BuildingClass::ChangeOwner`** `0x00448260` at `0x00448723`: sets to 1
  unconditionally after owner transfer completes
  ```
  00448713: MOV EBX, [ESI + 0x21c]            ; old owner
  00448719: PUSH ESI
  0044871a: LEA ECX, [ESP + 0x14]
  0044871e: CALL 0x006e6ab0                    ; some post-transfer call
  00448723: MOV byte ptr [ESI + 0x6e3], 0x1   ; ← flag set here
  ```

**Behavioral effects (all reduce the bounty from captured buildings):**

| Function | Address | Effect when `+0x6E3 != 0` |
|---|---|---|
| `SpawnSurvivors` | 0x00442DD9 | Adds 6 to the random roll range → significantly lower per-cell survivor spawn rate |
| `GetSurvivorCount` (vtable+0x2D0) | 0x00451397 | Doubles SurvivorDivisor → halves survivor count |
| `GetSurvivorInfantryType` (vtable+0x30C) | 0x0044EB15 | Disables the 25% ConYard→Engineer bonus |

### Rust implementation implication

When a building changes owner (engineer capture, mind control, spy effect if
applicable), set `+0x6E3 = true` so subsequent sell/destroy events yield
reduced crew. This is an anti-exploit: players can't capture an enemy
building then sell it for a full crew payout.

## 2. Type+0x154B = `HasSpotlight=` (INI key)

**Evidence:** `BuildingTypeClass::ReadINI` at `0x0045FEE5`-`0x0045FF13`:

```
0045fee5: MOV CL, byte ptr [EBP + 0x154b]    ; load prior/default
0045feeb: PUSH ECX
0045feec: PUSH 0x81aea0                       ; INI key string ptr
0045fef1: PUSH ESI
0045fef2: MOV ECX, EDI                        ; INI class ptr
0045fef4: CALL 0x005295f0                     ; INIClass::ReadBool (signature: takes default)
0045ff0d: MOV byte ptr [EBP + 0x154b], AL    ; store result
```

String at `0x0081AEA0` = **"HasSpotlight"** (confirmed via `search_strings`).

Confirms the gating condition for `+0x600 BuildingLightClass*` creation in
Unlimbo. In Round 1 I found the gate but had not confirmed the INI keyword.

## 3. Gap-gen 4-state machine lives at BuildingClass+0x220

Master doc previously described the 4-state machine (0 Inactive, 1 Expanding,
2 Active, 3 Contracting) but did not state the offset explicitly. My Round-1
report noted a possible offset mismatch.

**Evidence:** `BuildingClass::UpdateGapGenerator_Tick` at `0x00454DB0`:

```
00454e02: MOV EAX, [ESI + 0x220]              ; read state DWORD
00454e08: CMP EAX, 0x1                         ; Expanding?
00454e85: MOV dword ptr [ESI + 0x220], 0x2    ; → Active
00454ea9: CMP EAX, 0x3                         ; Contracting?
00454f2f: MOV [ESI + 0x220], EBP              ; → Inactive (EBP==0)
00455012: CMP dword ptr [ESI + 0x220], 0x2    ; state==Active check
00455035: MOV EAX, [ESI + 0x220]              ; state==Inactive check
```

`+0x220` stores the DWORD-sized 4-state enum. **Do not confuse with `+0xBC`**,
which is the MissionClass-level sub-state (used by Mission_Selling 3-state
machine — a completely separate thing).

## 4. vtable+0x45C / +0x460 resolved (TechnoClass cloak machinery)

Round-1 flagged vtable+0x460 as "inherited TechnoClass slot 280, purpose
unknown." Resolved via BuildingClass vtable read at `0x007E431C`:

| Slot | Offset | Address | Function |
|---|---|---|---|
| 168 | 0x2A0 | `0x00457770` | **BuildingClass::CanCloak** |
| 169 | 0x2A4 | `0x004578C0` | **BuildingClass::ShouldUncloak** |
| 279 | 0x45C | `0x007036C0` | **TechnoClass::StartUncloaking** |
| 280 | 0x460 | `0x00703770` | **TechnoClass::StartCloaking** |

### The `UpdateGapGenerator_Tick` post-state hook

```c
if (gap_state == 2 /* Active */ && this->ShouldUncloak()) {
    this->StartUncloaking();  // slot 279
}
if (gap_state == 0 /* Inactive */ && this->CanCloak()) {
    this->StartCloaking();    // slot 280
}
```

**Interpretation:** The gap generator's active/inactive transitions drive the
building's *own* cloak state (if it's Cloakable). Active GapGen forces
uncloak; Inactive lets it re-cloak. **Not** a dedicated UnInit — just the
regular TechnoClass cloak state machine riding on the GapGen state changes.

**Active in YR:** Conditional. `Cloakable=yes` is not set on any retail YR
building; the code is reachable but inert in standard play. The `ShouldUncloak`
check would short-circuit for non-cloakable buildings.

## 5. +0x4DC / +0x4F0 / +0x4F4 are TechnoClass SoundEvent fields

Round-1 flagged these as inherited fields with unknown purpose. Resolved by
tracing `CALL 0x004060F0` cross-references.

**`0x004060F0` = `SoundEvent::SetLoopHandle(int* this, int audio_event, int loop)`:**

```c
void SoundEvent__SetLoopHandle(int *this, int audio_event, int loop) {
    if (this[3] == &DAT_0087E294) {   // vtable sig check
        this[0] = audio_event;
        this[2] = loop;
        if (audio_event != 0) {
            this[1] = *(int *)(audio_event + 0x138);
            if (loop == 0) {
                this[2] = *(int *)(audio_event + 0x24);
            }
            *(int **)(audio_event + 0x278) = this;  // back-ref
        }
    }
}
```

Cross-referenced from: `VocClass::PlayAtPos`, `VocClass::PlayAt`,
`AnimClass::UpdateLoopingSound`, `Process_QueuedEvents`, `RadarClass::*`,
`BuildingClass::Sell`, `UnitClass::Deploy`.

### BuildingClass field layout (inherited from TechnoClass)

| Offset | Size | Purpose |
|---|---|---|
| +0x4DC..+0x4EB | 16 bytes | **Embedded SoundEvent struct** (4 DWORDs: handle/audio_ref/loop_data/vtable_sig @ DAT_0087E294) |
| +0x4EC..+0x4EF | 4 bytes | Part of 5-DWORD MOVSD.REP copy block; purpose not isolated (may be padding or SoundEvent extension) |
| +0x4F0 | 4 bytes | Sound loop handle #1 (-1 = none) |
| +0x4F4 | 4 bytes | Sound loop handle #2 (-1 = none) |

### Inheritance during MCV undeploy

`Mission_Selling` state 2 at `0x0044A0D4`:
```
MOVSD.REP × 5                  ; copy +0x4DC..+0x4EF (20 bytes)
MOV [dst +0x4F0], [src +0x4F0] ; copy loop handles
MOV [dst +0x4F4], [src +0x4F4]
call SoundEvent::SetLoopHandle(&src[+0x4DC], 0, 0)  ; detach source
MOV [src +0x4F0], -1           ; invalidate source handles
MOV [src +0x4F4], -1
```

MCV inherits the sound loops seamlessly on undeploy. Consistent with master
doc's "Inherits sound loops" claim.

## 6. ExitObject (0x00443C60) — Top-Level Dispatch Survey

Partial survey — full 6724-byte function is too large for one pass. Documented
here: the top-level RTTI dispatch, entry checks, and major sub-paths.

### Signature and entry

```c
undefined4 __thiscall BuildingClass::ExitObject_Main(
    int *this,
    int *object_to_exit,    // param_2
    undefined4 flag          // param_3
);
```

Returns:
- **0** = exit failed (obstructed, no path, no valid cell)
- **1** = partial/retry (caller should try again next tick)
- **2** = exit successful

### Entry

```c
if (param_2 == NULL) return 0;
param_2->field_0x3d5 = 1;                    // "spawning-out" flag on the exiting object
kind = param_2->vt_kind();                    // vtable slot 11 (offset 0x2C)
```

### RTTI/Kind enum (vtable slot 11 — NOT the standard WhatAmI slot 8)

Observed in ExitObject's switch:

| Value | Class | Path |
|---|---|---|
| 1 | `UnitClass` (vehicles) | falls through to common tail |
| 2 | `AircraftClass` | dedicated aircraft case |
| 6 | `BuildingClass` | dedicated building case (building spawning a building, e.g., Cloning Vats) |
| 0xF | (likely `InfantryClass`) | shares common tail with case 1 |
| other | default | returns 0 |

`BuildingClass::vt_func_11` at `0x00459EC0` just returns `6` (constant). The
value is class-specific; this is a "Kind" tag distinct from the detailed
WhatAmI RTTI (which uses 0x10/0x28/0x07 etc).

### Case 2: Aircraft exit

```c
HouseClass::AI_EconomyStateMachine(2);
Owner[+0x5658] = -1;                         // clear AI aircraft-request slot

if (some_precondition) {
    // Try exit via helipad dock coord or Type.ExitCoord
    ...
    if (Unlimbo succeeded) {
        // set facing (Random if ExitCoord is default, else derive from
        // map cell direction)
        aircraft->vtable[0x480](cell);        // SetDestination/SetPath
        aircraft->vtable[0x1F0](2);           // Queue_Mission(MOVE)
        goto LAB_00444971;                    // success return 2
    }
    goto LAB_00444EDE;                        // fail return 0
}
```

### Case 6: Building exit (building-from-building)

```c
HouseClass::AI_EconomyStateMachine(6);
Owner[+0x564C] = -1;                         // AI building-request slot

pending_entry = FUN_0042EB20();              // look up pending build slot
// Compute target cell from BuildingTypeClass.XYZ and map constraints
// Special-case: ConYard spawning a building uses the pending build queue
// at Owner[+0x5704..+0x5714]

iVar5 = BuildingTypeClass::CanBePlacedAt();
switch (iVar5) {
    case 0: try alternate placement; if Unlimbo works, remove from queue, return 2
    case 1: queue dequeue + return 1 (should retry)
    case 2: // alt location ok
    default: return 0
}
```

### Common tail (cases 1, 0xF, and fall-through from other paths)

Series of type-based conditionals — each checks a BuildingTypeClass flag
and takes a specialized path:

1. **Hospital/Armory/WeaponsFactory precondition check**
   ```c
   if (!Type+0x16C1 [Hospital] && !Type+0x16C2 [Armory] && !Type+0x16BD [WeaponsFactory]) {
       if (!FUN_0065ADC0()) return 1;   // some obstruction check
   }
   ```

2. **AI economy state (non-Hospital/Armory)**
   ```c
   if (!Type+0x16C1 && !Type+0x16C2) {
       HouseClass::AI_EconomyStateMachine(kind);
       if (kind == 1)    Owner[+0x5650] = -1;    // AI unit-request
       if (kind == 0xF)  Owner[+0x5654] = -1;    // AI infantry-request(?)
   }
   ```

3. **Refinery/Weeder** (`Type+0x16BB` or `Type+0x16BC`)
   ```c
   if (kind != 1) {
       // Not a unit → just place (e.g., harvester from refinery)
       object->vtable[0x174]();   // some place/init call
       return 0;
   }
   // Unit unloading from refinery: compute dock coord using
   // g_DirectionOffsets + center + DAT_0089F698 offset
   // Unlimbo, set facing (0x8000), assign mission 0xA (Move/Dock)
   goto LAB_00444EDE;
   ```

4. **WeaponsFactory (vehicle exit)** — `!Type+0x16BD` fails here; the code
   drops into the complex cell-offset exit path with barracks-variant handling
   (Type+0x16E4/16E5/16E6 = GDI/NOD/Yuri flags). Output: Unlimbo unit facing
   away from foundation edge, queue Mission 2 (Move), TechnoClass::SetGhostCell
   for AI.

5. **Barracks / ConYard / Hospital / Armory / Cloning** path
   (Type.Factory==0x10 [Infantry] OR Hospital OR Armory OR Cloning):
   - Similar cell-offset math but infantry-specific
   - **Cloning Vats hook at `0x004449FB`**: if building is a plain Barracks
     (Factory==0x10) AND NOT itself a Cloning Vat (Type+0x16AC == 0),
     iterate HouseClass+0xFC (Cloning Vats list), call each vat's
     `vtable[0x100]` to spawn a duplicate infantry. Confirms master doc
     section 20.
   - Barracks variant exits:
     - `Type+0x16E4 GDIBarracks=` → use (+1,+2) cell offset from foundation
     - `Type+0x16E5 NODBarracks=` → use (+2,+2)
     - `Type+0x16E6 YuriBarracks=` → use (+2,+1)
   - Each path applies foundation-based offset adjustments

### What's NOT in ExitObject

- **5-state vehicle gate machine** — described in master doc section 10. Not
  a single state machine inside this function; it's spread across:
  - `BuildingClass::ToggleGate` (0x00443B90, vtable slot 242) — handles gate
    animation
  - `BuildingClass::Mission_RepairAndProduce` — drives the gate open/close
    timing relative to production completion
  - `UnitClass` locomotor piggyback logic — the actual "drive out of WF" movement
  
  The 5 conceptual phases (init → clear bib → drive out → wait → close gate)
  are a high-level description of the combined behavior, not a literal state
  machine enum in any one function.

- **Naval exit** — references in master doc to "special water cell finder"
  not yet traced here. Likely a sub-branch of the WF exit path gated by
  `Type+0xCCE Naval=` that tries 3 adjacent water cells.

### Call graph — vtable dispatches observed in ExitObject

Called on `object_to_exit`:
- `vtable[0x2C]` → Kind query
- `vtable[0xD8]` → `Unlimbo(cell, facing)`
- `vtable[0x174]` → some place/init (Refinery case)
- `vtable[0x1B4]` → `SetLocation/SetPath`
- `vtable[0x1E8]` → `Queue_Mission(enum, flag)`
- `vtable[0x1F0]` → alternate Queue_Mission variant
- `vtable[0x480]` → `SetDestination(cell, flag)`
- `vtable[0x88]` → `GetCoord / DistanceTo`
- `vtable[0x100]` → Cloning Vats dispatch (hook)
- `vtable[0x124]` → `SetLooped(flag)` or facing setup
- `vtable[0x278]` → `Assign_Destination` variant (called with 0x17 BARRACKS ENUM and 3, etc.)

---

## Summary of Round-2 Findings

| # | Question | Status |
|---|---|---|
| 1 | +0x6E3 purpose | ✓ OwnershipChanged flag (set in ChangeOwner, reduces crew bounty) |
| 2 | Type+0x154B INI key | ✓ `HasSpotlight=` (VA 0x0081AEA0) |
| 3 | Gap-gen state offset | ✓ BuildingClass+0x220 (DWORD) — not +0xBC |
| 4 | vtable+0x460 purpose | ✓ TechnoClass::StartCloaking (paired with slot 279 StartUncloaking) |
| 5 | +0x4DC/+0x4F0/+0x4F4 | ✓ TechnoClass SoundEvent + loop handles |
| 6 | ExitObject top-level survey | Partial — RTTI dispatch and major branches documented; full path-by-path tracing deferred |

### Remaining unknowns after this round

1. **Kind enum values** for vtable slot 11 — 0xF observed but not confirmed
   (probably InfantryClass; needs a quick check of InfantryClass vtable).
2. **ExitObject vehicle path details** — bib clearing, gate coordination,
   direction math for pulling out of WeaponsFactory vs Naval Yard.
3. **Naval exit branch** — water-cell search logic inside ExitObject.
4. **`FUN_0065ADC0` precondition** — called before non-Hospital/Armory exit,
   returns bool; purpose not resolved (likely checks if exit path is clear).
5. **Pending build queue format** at `HouseClass+0x5704/+0x5708/+0x5714` —
   16-byte entries; tuple format and consumer not fully traced.

### Master-doc updates from this round

1. **Section 2 (BuildingClass layout)** — add rows:
   - `+0x220 | int | GapGenerator state (0 Inactive, 1 Expanding, 2 Active, 3 Contracting)`
   - `+0x4DC..+0x4EF | bytes[20] | SoundEvent struct (inherited from TechnoClass)`
   - `+0x4F0 | int | Sound loop handle #1 (-1=none)`
   - `+0x4F4 | int | Sound loop handle #2 (-1=none)`
   - `+0x6E3 | bool | OwnershipChanged flag (set by ChangeOwner, reduces survivor bounty)`
2. **Section 3 (BuildingTypeClass layout)** — add row:
   - `+0x154B | bool | HasSpotlight= (gates +0x600 BuildingLightClass* allocation)`
3. **Section 14 (Gap Generator)** — state lives at `BuildingClass+0x220`; the
   4-state cycle is exact as previously documented but the offset was missing.
4. **Section 17 Mission_Selling MCV undeploy** — sound-loop inheritance
   details now concrete (20-byte SoundEvent + 2 DWORDs).

---

## Sources

### Functions decompiled or disassembled this round

- `0x0043B740` — BuildingClass::Constructor (+0x6E3 init)
- `0x00442D90` — BuildingClass::SpawnSurvivors (+0x6E3 effect on bounty range)
- `0x00448260` — BuildingClass::ChangeOwner (+0x6E3 setter)
- `0x00451330` — GetSurvivorCount (+0x6E3 doubles divisor)
- `0x00454DB0` — BuildingClass::UpdateGapGenerator_Tick (+0x220 state confirmation)
- `0x0045FE50` — BuildingTypeClass::ReadINI (HasSpotlight= parse)
- `0x004060F0` — SoundEvent::SetLoopHandle (cross-ref verification)
- `0x00443C60` — BuildingClass::ExitObject_Main (top-level survey)
- `0x00459EC0` — BuildingClass::vt_func_11 (returns 6)

### Vtable reads

- `0x007E431C` (BuildingClass vtable slot 280)
- `0x007E4318` (slot 279)
- `0x007E415C` (slot 168)
- `0x007E4160` (slot 169)

### Strings verified

- `0x0081AEA0` = "HasSpotlight"

### Cross-references

- `0x004060F0` cross-refs = 11 callers across VocClass, AnimClass,
  RadarClass, BuildingClass::Sell, UnitClass::Deploy — confirms SoundEvent
  identity
