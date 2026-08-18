# FootClass::AI Deep Dive — Ghidra Report

**Address:** `0x4DA530`
**Size:** 722 instructions, 138 basic blocks, cyclomatic complexity 130
**Confidence:** 90%+ overall, 95%+ for IPiggyback swap sequence
**Date:** 2026-03-22

## Summary

`FootClass::AI()` is the per-tick update function for every mobile unit in the game.
It is called from derived class AI methods:
- `UnitClass::AI` (0x7360C0) calls `FootClass::AI` directly at 0x73647B
- `InfantryClass::AI` (0x51BAB0) calls `FootClass::AI` at 0x51BC9F
- AircraftClass::AI also calls it (via vtable inheritance)

The function handles 10 major subsystems in this order:

1. **Parent class AI** — calls `TechnoClass::AI_Update` (0x6F9E50)
2. **Tiberium self-heal** — heal on tiberium cells at configurable interval
3. **Veteran promote check** — elite upgrade + on-screen anim
4. **ILocomotion::Process** — drives locomotor state machine each tick
5. **Movement counter** — tracks frame counter for movement timing
6. **Rank-up / falling anim** — spawns veteran/elite promotion and falling anims
7. **IPiggyback locomotor swap** — the chrono miner swap mechanism
8. **Try enter transport** — attempts to board a transport if applicable
9. **Team AI dispatch** — calls Team::AI via team pointer
10. **Idle scatter** — every 64 frames, scatter idle units

## FootClass Struct Layout (fields accessed in AI)

All offsets relative to FootClass `this` pointer (param_1 in Ghidra).

| Byte Offset | Dword Index | Field | Notes |
|-------------|-------------|-------|-------|
| 0x0000 | [0] | vtable* | Primary vtable pointer |
| 0x0004 | [1] | secondary_vtable_4 | |
| 0x0008 | [2] | secondary_vtable_8 | |
| 0x000C | [3] | secondary_vtable_12 | |
| 0x0081 | — | byte flag | Limbo/off-map flag (if set, skip tib heal + loco process) |
| 0x008C | — | byte flag | "is alive" status byte |
| 0x008D | — | IsDeployed | Deployed state flag |
| 0x008E | — | WasDeployed | Previous deployed state (for change detection) |
| 0x008F | — | DeployedAnimFlag | Whether deploy anim should play |
| 0x0090 | [0x24] | Health/IsActive | Checked for early return (if 0, return immediately) |
| 0x009C | [0x27] | Location_X | Leptons X |
| 0x00A0 | [0x28] | Location_Y | Leptons Y |
| 0x00A4 | [0x29] | Location_Z | Leptons Z |
| 0x021C | [0x87] | Owner (HouseClass*) | Owner house pointer |
| 0x0260 | [0x98] | SightRange related | Used in fog border update |
| 0x0264 | [0x99] | LastHeight | Previous height for fog updates |
| 0x02A8 | [0xAA] | SomeTargetPtr | If nonzero, checks TypeClass+0x692 |
| 0x02B4 | [0xAD] | AnotherTarget | Related to movement scatter |
| 0x03CD | — | IsCrashing | Crashing/falling state flag |
| 0x03CE | — | WasCrashing | Previous crashing state |
| 0x03D5 | — | IsOnMap | Placed-on-map flag |
| 0x0425 | — | IsFalling | Falling state flag |
| 0x0426 | — | WasFalling | Previous falling state |
| 0x0500 | [0x140] | TransportTarget | Pointer to transport to enter |
| 0x0520 | [0x148] | TypeClassPtr | TechnoTypeClass pointer (also via vtable+0x84) |
| 0x0538 | [0x14E] | MovementCounter | Incremented each tick when locomotor is processing |
| 0x053C | [0x14F] | VetAnimActive | Whether veteran anim is currently active |
| 0x0540 | [0x150] | VetAnimTimer | Countdown timer for veteran anim |
| 0x05A4 | [0x169] | NavCom/DestTarget | Navigation target ID (if nonzero, don't scatter) |
| 0x0644 | [0x191] | — | Timer base frame for visibility |
| 0x0648 | [0x192] | — | |
| 0x064C | [0x193] | — | |
| 0x0650 | [0x194] | — | |
| 0x065C | [0x197] | FogUpdateFrame | Last frame fog border was updated |
| 0x0660 | [0x198] | — | |
| 0x0664 | [0x199] | FogUpdateDelay | Frames until next fog update (set to 0xF = 15) |
| **0x0674** | **[0x19D]** | **ILocomotion*** | **Active locomotor COM pointer** |
| 0x0684 | [0x1A1] | byte flags | Bit 7 checked for early return in InfantryClass::AI |
| 0x0694 | [0x1A5] | Team* | Pointer to team this unit belongs to |
| 0x06AD | — | IsDeploying | Deployment in-progress flag (blocks loco swap) |
| 0x06B3 | — | SomeTickFlag | Cleared to 0 at start of every AI tick |
| 0x06B4 | [0x1AD] | IPiggybackChecked | Cleared to 0 after IPiggyback check |

## The IPiggyback Swap Sequence (0x4DAE5F - 0x4DAEC6)

This is the critical code that restores a piggybacked locomotor. It runs every tick
for every mobile unit, but the swap only happens when `Is_Ok_To_End()` returns true.

### Assembly-level walkthrough

```asm
; Step 1: Get current ILocomotion* pointer
004dae5f: MOV EAX,[ESI+0x674]         ; EAX = this->Locomotor (ILocomotion*)
004dae65: LEA EBP,[ESI+0x674]         ; EBP = &this->Locomotor
004dae6b: XOR EDI,EDI                  ; EDI = 0 (IPiggyback* result)
004dae6d: CMP EAX,EBX                  ; if Locomotor == NULL
004dae6f: JZ 0x4daec6                  ;   skip everything

; Step 2: QueryInterface for IPiggyback
004dae71: MOV ECX,[EAX]               ; ECX = ILocomotion vtable
004dae73: LEA EDX,[ESP+0x14]          ; EDX = &result (output param)
004dae77: PUSH EDX                     ; push &result
004dae78: PUSH 0x819088                ; push &IID_IPiggyback
004dae7d: PUSH EAX                     ; push ILocomotion* this
004dae7e: CALL [ECX]                   ; QueryInterface(this, IID, &result)

; Step 3: Process QueryInterface result
004dae80: MOV EDX,[ESP+0x14]          ; EDX = result from QI
004dae84: XOR ECX,ECX
004dae86: CMP EAX,EBX                  ; if HRESULT < 0 (FAILED)
004dae88: SETL CL                      ;   CL = 1
004dae8b: DEC ECX                      ;   ECX = 0 (mask = 0)
004dae8c: AND ECX,EDX                  ; else ECX = result
004dae90: MOV EDI,ECX                  ; EDI = IPiggyback* (or NULL if failed)

; Step 3b: Assert on unexpected failure (not E_NOINTERFACE)
004dae92: JGE 0x4daea1                 ; skip if SUCCEEDED
004dae94: CMP EAX,0x80004002           ; E_NOINTERFACE
004dae99: JZ 0x4daea1                  ; skip assert if E_NOINTERFACE
004dae9b: PUSH EAX
004dae9c: CALL GameDebugLog__Assert    ; assert on unexpected QI failure

; Step 4: Call Is_Ok_To_End
004daea1: CMP EDI,EBX                  ; if IPiggyback* == NULL
004daea3: JZ 0x4daec6                  ;   skip
004daea5: MOV EDX,[EDI]               ; EDX = IPiggyback vtable
004daea7: PUSH EDI                     ; push IPiggyback* this
004daea8: CALL [EDX+0x14]             ; Is_Ok_To_End()
004daeab: TEST AL,AL
004daead: JZ 0x4daec6                  ; if false, skip swap

; Step 5: Release old locomotor
004daeaf: MOV EAX,[EBP]               ; EAX = current ILocomotion*
004daeb2: CMP EAX,EBX
004daeb4: JZ 0x4daebc                  ; skip if NULL
004daeb6: MOV ECX,[EAX]               ; ECX = ILocomotion vtable
004daeb8: PUSH EAX                     ; push ILocomotion* this
004daeb9: CALL [ECX+0x8]              ; ILocomotion::Release()

; Step 6: Clear locomotor pointer
004daebc: MOV [EBP],EBX               ; this->Locomotor = NULL

; Step 7: End_Piggyback writes restored locomotor into cleared slot
004daebf: MOV EDX,[EDI]               ; EDX = IPiggyback vtable
004daec1: PUSH EBP                     ; push &this->Locomotor (output param)
004daec2: PUSH EDI                     ; push IPiggyback* this
004daec3: CALL [EDX+0x10]             ; End_Piggyback(this, &this->Locomotor)
```

### Key observations

1. **No Link_To_Object call** on the restored locomotor. The restored loco was already
   linked to the unit when `Begin_Piggyback` stored it. The link persists through piggybacking.

2. **No AddRef** on the restored locomotor. `End_Piggyback` simply moves the pointer
   from the piggyback's internal storage to `FootClass+0x674`. The ownership transfers
   directly.

3. **Release is called on the OLD (piggyback) locomotor** before End_Piggyback.
   This drops the piggyback locomotor's reference count. If it hits 0, it destroys.

4. **IPiggyback* is Released at the very end of FootClass::AI** (at 0x4DAEFD), after
   all other processing is done. This is the Release that matches the AddRef from
   QueryInterface.

5. **E_NOINTERFACE (0x80004002) is expected** for locomotors that don't support
   IPiggyback (e.g., DriveLocomotion when it's not piggybacking anything). The code
   silently ignores this. Any other QI failure triggers an assert.

## Is_Ok_To_End Conditions

### TeleportLocomotionClass (0x719F30)
Returns true only when ALL of:
1. Inner End_Piggyback returns false (no nested piggyback)
2. Timer at +0x30 is nonzero (warp has been initiated)
3. Flag at +0x1D == 0 (not in warp transition)
4. `FootClass+0x27C` (ChronoInTransit) == 0
5. Warp state at +0x20 == 0 (state machine completed)
6. `FootClass+0x6AD` (IsDeploying) == 0

### DriveLocomotionClass (0x4AF970)
Returns true only when ALL of:
1. Inner End_Piggyback returns false
2. Stored locomotor at +0x50 exists
3. Flag at +0x4D != 0 (ready-to-restore flag)
4. `FootClass+0x6AD` (IsDeploying) == 0

## ILocomotion COM Interface Vtable Layout

Verified from TeleportLocomotionClass ILocomotion vtable at 0x7F5000:

| Offset | Slot | Method | TeleportLoco Implementation |
|--------|------|--------|----------------------------|
| +0x00 | 0 | QueryInterface | 0x0071A160 |
| +0x04 | 1 | AddRef | 0x0071A170 |
| +0x08 | 2 | Release | 0x0071A180 |
| +0x0C | 3 | Link_To_Object | 0x0055A710 (shared) |
| +0x10 | 4 | Is_Moving | 0x00718080 |
| +0x14 | 5 | Destination | 0x007180A0 |
| +0x18 | 6 | Head_To_Coord | 0x0055ACA0 (shared) |
| +0x1C | 7 | Stop_Moving | 0x0055ABF0 (shared) |
| +0x40 | 16 | **Process** | 0x007192F0 (StateMachineTick) |
| +0x80 | 32 | **Is_Moving_Now** | 0x004B6610 (thunk to slot 4) |

**Process (slot 16, +0x40)** is called every tick from FootClass::AI to drive the
locomotor's state machine. For TeleportLocomotionClass, this is the 8-state chrono
warp state machine.

**Is_Moving_Now (slot 32, +0x80)** is checked throughout FootClass::AI to determine
if the unit is currently in motion. Used to gate movement counter updates, scatter
behavior, and various conditional checks.

## IPiggyback COM Interface Vtable Layout

Verified from TeleportLocomotionClass IPiggyback vtable at 0x7F4FDC:

| Offset | Slot | Method | TeleportLoco | DriveLoco (0x7E7E8C) |
|--------|------|--------|--------------|----------------------|
| +0x00 | 0 | QueryInterface | 0x0071A190 | 0x004B4DC0 |
| +0x04 | 1 | AddRef | 0x0071A1A0 | 0x004B4DD0 |
| +0x08 | 2 | Release | 0x0071A1B0 | 0x004B4DE0 |
| +0x0C | 3 | Begin_Piggyback | 0x00719E90 | 0x004AF8E0 |
| +0x10 | 4 | End_Piggyback | 0x00719EE0 | 0x004AF930 |
| +0x14 | 5 | Is_Ok_To_End | 0x00719F30 | 0x004AF970 |

## FootClass Virtual Method Table (vtable at 0x7E8C94)

Key overrides used in FootClass::AI:

| Offset | Method | Implementation |
|--------|--------|---------------|
| +0x48 | GetCoords | ObjectClass::GetCoords (0x5F65A0) |
| +0x54 | IsHealthy | thunk to 0x5F6B90 (health check) |
| +0x5C | **AI** | **FootClass::AI (0x4DA530)** |
| +0x84 | GetTechnoType | TechnoClass::GetTechnoType_Trampoline (0x6F3270) |
| +0x16C | ReceiveDamage | FootClass::ReceiveDamage (0x4D7330) |
| +0x174 | Scatter | no-op in FootClass (0x5F43A0), overridden in subclasses |
| +0x1B8 | GetCell | BuildingClass::GetCell (0x41BEA0) |
| +0x1BC | GetOccupiedCell | ObjectClass::GetOccupiedCell (0x5F6960) |
| +0x1C8 | GetHeight | ObjectClass::GetHeight (0x5F5F40) |
| +0x1D4 | IsWarpingOut | TechnoClass::IsWarpingOut (0x70C5B0) |
| +0x1D8 | IsBeingWarped | TechnoClass::IsBeingWarped (0x70C5C0) |
| +0x1E8 | QueueMission | MissionClass::Queue_Mission (0x5B35E0) |
| +0x278 | TransmitRadio | RadioClass::Transmit_Radio (0x65AAA0) |
| +0x2B0 | IsNotAtDest | TechnoClass::IsNotAtDestination (0x70C620) |
| +0x480 | SetDestination | FootClass::Set_Destination_Internal (0x4D94B0) |

## What ELSE FootClass::AI Does (Beyond the Swap)

### 1. TechnoClass::AI Parent Call (line 1)
First thing: calls `TechnoClass::AI_Update()` at 0x6F9E50. This handles:
- Temporal (chrono erase) visual updates
- Gap generator visual updates
- Health bar smoothing
- Shield/Iron Curtain timer countdown
- Cloaking state updates
- Turret rotation
- Weapon reload timers

### 2. Tiberium Self-Heal (lines ~5-25)
Every `Rules+0x1808` frames (TiberiumHealDelay):
- Check TypeClass+0xD37 (Tiberium immune flag)
- Check IsHealthy (vtable+0x54)
- Check not in limbo (offset 0x81)
- Get cell at current position, read tiberium amount
- If > 0, apply heal via ReceiveDamage (vtable+0x16C) using Rules+0x1834 heal rate

### 3. On-Screen Veteran Check (lines ~26-35)
- If not deployed (offset 0x3D5) and TypeClass+0xD6A flag set (trainable)
- Call TechnoClass::IsOnScreen (0x578540)
- Set deployed flag if on screen

### 4. ILocomotion::Process (lines ~36-50)
The core locomotor tick:
- If ILocomotion* at +0x674 is NULL, assert
- Call `ILocomotion::Process()` (vtable+0x40 on the locomotor)
- This drives the locomotor's state machine (e.g., chrono warp phases)
- After Process, check if unit died (Health == 0), if so return immediately

### 5. Movement Counter & Scatter Logic (lines ~50-130)
Complex movement tracking:
- `param_1[0x14E]` = movement counter, incremented when conditions met
- Checks Is_Moving_Now (ILocomotion vtable+0x80), IsWarpingOut, IsBeingWarped
- TypeClass+0x294 = movement scatter interval
- TypeClass+0x298 = secondary scatter interval
- TypeClass+0x6AD = IsDeploying flag
- Tracks whether movement counter changed to decide scatter behavior

### 6. Rank-Up / Falling Anim System (lines ~130-280)
Monitors state transitions for deployed, crashing, and falling states:

**Deployed state** (offset 0x8D vs 0x8E):
- On change to deployed: spawn deploy anim if TypeClass+0x54C != -1
- On change from deployed: release existing anim

**Crashing state** (offset 0x3CD vs 0x3CE):
- On change to crashing: spawn crash anim (TypeClass+0x554),
  spawn secondary crash anim (TypeClass+0x548), or fall back to Rules+0x208
- On change from crashing: release anim

**Falling state** (offset 0x425 vs 0x426):
- On change to falling: release old anim, spawn falling anim (TypeClass+0x550)
  if player-controlled (via TechnoClass::IsPlayerControlled 0x50B6F0),
  spawn secondary falling anim (TypeClass+0x544)
- On change from falling: release anim

### 7. AnimClass::SpawnDetached (line ~287)
Called with current location to update detached animations.

### 8. Idle Scatter (lines ~288-305)
Every 64 frames (`g_CurrentFrameCounter & 0x3F == 0x3F`):
- If no NavCom target (param_1[0x169] == 0)
- If not selected (param_1[0x23] == 0)
- If house not observer (GetOccupiedCell -> check +0x11C)
- If not at destination (vtable+0x2B0)
- If height == 0 (vtable+0x1C8)
- Call Scatter (vtable+0x174) with zero coord and force flag

### 9. IPiggyback Swap (lines ~306-365)
See detailed analysis above. This is the mechanism that swaps between:
- TeleportLocomotionClass (active during chrono warp)
- DriveLocomotionClass (piggybacked, restored after warp completes)

### 10. Post-Swap Cleanup (lines ~366-383)
- Clear flag at +0x6B4
- If not being warped (vtable+0x1D8), try to enter transport (FootClass::TryEnterTransport 0x70D7E0)
- If has team (param_1[0x1A5]), call Team's AI method
- Release IPiggyback pointer (IUnknown::Release on QI result)

## Labeled Functions

| Address | Name | Confidence |
|---------|------|------------|
| 0x4DA530 | FootClass__AI | 95% (plate comment set) |
| 0x6F9E50 | TechnoClass__AI_Update | 95% (already labeled) |
| 0x7DC720 | GameDebugLog__Assert | 90% |
| 0x70C620 | TechnoClass__IsNotAtDestination | 85% |
| 0x70D7E0 | FootClass__TryEnterTransport | 85% |
| 0x746D80 | UnitClass__IsFalling | 80% (returns byte at +0x6E0) |
| 0x6E53A0 | TechnoClass__ProcessCellAction | 70% (needs more study) |
| 0x70AF50 | TechnoClass__UpdateVeterancyAnim | 80% |
| 0x70B1D0 | TechnoClass__ClearVeterancyAnim | 80% |
| 0x65C780 | Random__Next | 90% (MT-style RNG) |
| 0x405D40 | AnimClass__Detach | 85% |
| 0x406060 | AnimClass__Release | 85% |
| 0x578540 | TechnoClass__IsOnScreen | 85% |
| 0x4D7330 | FootClass__ReceiveDamage | 85% |
| 0x51BAB0 | InfantryClass__AI | 95% |
| 0x7360C0 | UnitClass__AI | 95% |
| 0x4B6610 | ILocomotion__Is_Moving_Now_Thunk | 90% |
| 0x7509E0 | AnimClass__SpawnAtCoord | 75% |
| 0x750D40 | AnimClass__SpawnDetached | 75% |

## LocomotionClass Base Layout (verified from constructor 0x55A6C0)

| Offset | Field | Notes |
|--------|-------|-------|
| +0x00 | IUnknown vtable* | |
| +0x04 | ILocomotion vtable* | |
| +0x08 | Linked FootClass* (copy 1) | Set by Link_To_Object |
| +0x0C | Linked FootClass* (copy 2) | Set by Link_To_Object |
| +0x10 | flags byte (init 1) | |
| +0x11 | flags byte (init 1) | |
| +0x14 | (init 0) | |

**Link_To_Object** (0x55A710) stores the owner FootClass pointer at BOTH +0x08 and +0x0C
(relative to base). When Is_Ok_To_End reads `param_1 + -0xC` (where param_1 = IPiggyback*
= base+0x18), it gets `base+0x0C` = the linked FootClass pointer, then reads fields like
`FootClass+0x27C` (ChronoInTransit) and `FootClass+0x6AD` (IsDeploying).

### TeleportLocomotionClass extends LocomotionClass

| Offset | Field | Notes |
|--------|-------|-------|
| +0x00 | IUnknown vtable* | |
| +0x04 | ILocomotion vtable* | |
| +0x08 | Linked FootClass* | |
| +0x0C | Linked FootClass* | |
| +0x18 | IPiggyback vtable* | param_1 in Is_Ok_To_End |
| +0x1C | Dest coord X | |
| +0x20 | Dest coord Y | |
| +0x24 | Dest coord Z | |
| +0x28 | Source coord X | |
| +0x2C | Source coord Y | |
| +0x30 | Source coord Z | |
| +0x34 | byte flag | |
| +0x35 | byte flag | |
| +0x38 | Warp state (int) | Relative to IPiggyback: param_1+0x20 |
| +0x3C | Timer (int) | |
| +0x44 | field_44 | |
| +0x48 | Stored ILocomotion* | End_Piggyback reads from IPiggyback+0x30 = base+0x48 |

Note: `End_Piggyback` at 0x719EE0 accesses `param_1 + 0x30` where param_1 is IPiggyback*
(base+0x18), so the stored locomotor is at base+0x18+0x30 = base+0x48.

## COM Interface Pointer Adjustment in QueryInterface

When FootClass::AI calls `QueryInterface(ILocomotion*, &IID_IPiggyback, &result)`:
- The ILocomotion* points to base+4 in the locomotor
- The ILocomotion vtable slot 0 is an adjuster thunk that subtracts 4 from the `this`
  parameter and jumps to the real QI
- The real QI (0x719E30) checks if the IID matches IPiggyback
- If yes, it returns `base + 0x18` (= param_1 + 6) as the IPiggyback*
- It also calls AddRef on the locomotor

This is standard COM multiple interface implementation with adjuster thunks.

## Implementation Notes for Rust Engine

### The swap sequence in Rust
```rust
// Every tick in FootClass::AI:
if let Some(loco) = &self.locomotor {
    if let Some(piggyback) = loco.query_piggyback() {
        if piggyback.is_ok_to_end() {
            // Release current (piggyback) locomotor
            let old_loco = self.locomotor.take();
            drop(old_loco);

            // Restore piggybacked locomotor
            if let Some(restored) = piggyback.end_piggyback() {
                self.locomotor = Some(restored);
                // Note: NO Link_To_Object call needed
                // Note: NO AddRef call needed
            }
        }
    }
}
```

### Key invariants
1. `FootClass+0x674` always holds exactly one ILocomotion reference
2. The piggyback locomotor stores the old locomotor internally with an AddRef'd reference
3. During swap, the old (piggyback) loco is Released before End_Piggyback runs
4. End_Piggyback moves ownership (no AddRef) directly into FootClass+0x674
5. The restored loco's Link_To_Object binding persists through the piggyback cycle
6. `FootClass+0x6AD` (IsDeploying) blocks ALL locomotor swaps across all loco types
7. The IPiggyback QI result is Released at the very end of FootClass::AI
