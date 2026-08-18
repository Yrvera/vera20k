# BuildingClass::Update (Per-Tick AI) — Ghidra Report

**Date:** 2026-04-06
**Binary:** gamemd.exe
**Function:** `BuildingClass::Update` at `0x0043FB20`
**Vtable:** slot 23, offset `0x05C` in `vtable_BuildingClass` (`0x007E3EBC`)
**Size:** ~2,650 bytes (0x0043FB20–0x0044057A)
**Confidence:** HIGH — all addresses verified from decompilation, field offsets cross-referenced with INI reader

---

## Overview

`BuildingClass::Update` is the main per-tick function for every building. It is called once per
game frame for every living `BuildingClass` instance. The function handles power state transitions,
damage fire anims, cash generation, factory production ticks, auto-sell logic, repair logic,
animation state machine, delayed fire processing, bridge destruction, and gate/transport exit logic.

**Critical note on param_1 type:** In the decompilation, `this` is typed as `BuildingClass *`
(pointer). All field accesses like `this->field_0xNNN` are direct byte offsets. The `Type` field
(`this->Type`) is a pointer to `BuildingTypeClass`, and sub-offsets into Type are also direct byte
offsets (e.g., `this->Type + 0x155c`).

---

## Order of Operations (Tick Pipeline)

### Phase 1: Power State Check & Looping Sound Update
**Address:** `0x0043FB20`

```
1. cVar2 = this->vtable[0x350]()   // BuildingClass::IsOperational (IsPoweredUp)
   - Checks: HasPower, EMP, health>0, power ratio if Powered=yes, PoweredSpecial timer
   - Returns false if building offline/disabled

2. if (IsOperational == false  OR  mission == 0x12 (Selling)  OR  mission == 0x13 (Construction)):
     active_state = false
   else:
     active_state = true

3. if (active_state == this->field_0x6C8):   // same as last tick?
     // No change — just update looping sound if building has WorkingSound/NotWorkingSound
     if (Type+0xE80 != -1 || Type+0xE84 != -1):
       AnimClass::UpdateLoopingSound(this->Location)
   else:
     // State CHANGED this tick
     if (Type+0xE80 != -1 || Type+0xE84 != -1):
       FUN_00405FD0()   // Stop previous looping sound
       VocClass::PlayAt(this->field_0x6B4)  // Play transition sound
     BuildingClass::UpdateGapAndSpecialEffects(this)   // 0x004549B0
     this->field_0x6C8 = active_state   // Save new state
```

**Fields:**
- `this+0x6C8` (byte) — `LastActiveState`: cached power/operational state from last tick
- `Type+0xE80` (int) — `WorkingSound` (VocClass index, -1 = none)
- `Type+0xE84` (int) — `NotWorkingSound` (VocClass index, -1 = none)
- `this+0x6B4` — Sound-related field for transition playback

---

### Phase 2: Damage Fire Anim State Check
**Address:** `0x0043FBE8`

```
4. if (Type[0x157B] == false):   // damage-fire threshold selector; exact INI label unresolved
     threshold = RulesClass+0x1700   // ConditionYellow (double)
   else:
     threshold = RulesClass+0x1708   // ConditionRed (double)

5. damaged = (GetHealthRatio() <= threshold)

6. if (damaged != this->field_0x5E8):   // damage state changed?
     if (damaged == false):
       // REMOVE damage fires: loop 8 anim slots at field_0x5C8..0x5E4
       for i in 0..8:
         if (this->DamageFireAnims[i] != NULL):
           anim->Destroy()   // vtable+0xF8
           this->DamageFireAnims[i] = NULL
     else:
       BuildingClass::CreateDamageFireAnims(this)  // 0x0043C0D0
     this->field_0x5E8 = damaged
```

**Fields:**
- `this+0x5C8` .. `this+0x5E4` — array of 8 `AnimClass*` pointers for damage fire anims
- `this+0x5E8` (byte) — `IsDamagedState`: cached damage-fire state (0=not damaged, 1=damaged)
- `Type+0x157B` (bool) — damage-fire threshold selector / field label unresolved;
  older docs disagree on the INI label, so parser audit is needed
- `RulesClass+0x1700` (double) — `ConditionYellow` (health ratio threshold)
- `RulesClass+0x1708` (double) — `ConditionRed`

---

### Phase 3: Occupant Frame Counter (IdleRate)
**Address:** `0x0043FC82`

```
7. if (Type[0xCA1] == false):   // !HasTurretAnim (occupant-related)
     this->field_0x4A0 = 0
   else:
     this->field_0x4A0 = CDTimerClass::Remaining() & 0xFF  // IdleRate countdown
```

**Fields:**
- `Type+0xCA1` (bool) — `HasTurretAnim` / occupant firing anim flag
- `this+0x4A0` (byte) — `OccupantFrameCounter`

---

### Phase 4: Docked Aircraft/Unit Update
**Address:** `0x0043FCA0`

```
8. if (this->field_0x278 != NULL):   // Docked object (RadioClass contact)
     docked->vtable[0x5C]()   // Update the docked unit
```

**Fields:**
- `this+0x278` — Docked object pointer (RadioClass contact)

---

### Phase 5: Warping/Chrono Check — Early Return for Warping Buildings
**Address:** `0x0043FCAC`

```
9. isWarpingOut = this->vtable[0x1D4]()  // TechnoClass::IsWarpingOut
   isBeingWarped = this->vtable[0x1D8]()  // TechnoClass::IsBeingWarped

10. if (isWarpingOut || isBeingWarped):
      // Building is in chrono warp — skip normal update, spawn sparkle anims
      Type = this->Type

      if (Type+0x1580 == 0 || RulesClass+0x344 == 0):
        // No warp anim defined on type — use global default
        if (g_CurrentFrameCounter % 24 == 0 && RulesClass+0x344 != 0):
          create AnimClass(RulesClass+0x344, this->Location)
      else:
        // Type has specific warp-out anim locations (Type+0x1580 = count, +0x1588 = array)
        for i in 0..Type+0x1580:
          if ((g_CurrentFrameCounter + i) % 24 == 0 && RulesClass+0x344 != 0):
            coord = IsometricPixelToWorld(Type + 0x1588 + i*8)
            coord += this->GetRenderCoords()
            anim = AnimClass::Constructor(RulesClass+0x344, coord, ...)
            anim->ZAdjust = -200  // (0xFFFFFF38 = -200)

      if (this->field_0x2B4 != 0):  // has factory/transport
        goto ToggleGate   // exit early via gate toggle
      return
```

**Fields:**
- `Type+0x1580` (int) — Number of warp-out anim positions
- `Type+0x1588` — Array of isometric pixel coords for warp anims
- `RulesClass+0x344` (AnimTypeClass*) — `WarpOut` anim type

---

### Phase 6: ProduceCash Timer (Oil Derrick Per-Tick Money)
**Address:** `0x0043FD28`

```
11. timer_remaining = this->field_0x6D8   // ProduceCash remaining ticks
    if (this->field_0x6D0 == -1):   // timer not started
      goto check_timer_value
    else:
      elapsed = g_CurrentFrameCounter - this->field_0x6D0
      if (elapsed < timer_remaining):
        timer_remaining = timer_remaining - elapsed
        goto check_timer_value
      // else: timer expired, falls through

12. check_timer_value:
    if (timer_remaining == 1):   // Timer just expired (transition to 0)
      this->field_0x6D0 = g_CurrentFrameCounter   // restart timer
      this->field_0x6D8 = Type+0x1560             // ProduceCashDelay
      // Grant/deduct money if not parasited AND building is operational
      if (Owner->HouseTypeClass+0x1A6 == false && this->IsOperational()):
        amount = Type+0x155C   // ProduceCashAmount
        if (amount < 1):
          HouseClass::Spend_Money(-amount)   // negative = cost
        else:
          HouseClass::Add_Credits(amount)    // positive = income
```

**Fields:**
- `this+0x6D0` (int) — `ProduceCashTimer_StartFrame` (-1 = not started)
- `this+0x6D4` (int) — ProduceCash timer second field
- `this+0x6D8` (int) — `ProduceCashTimer_Duration` (remaining frames)
- `Type+0x1558` (int) — `ProduceCashStartup` (INI: ProduceCashStartup)
- `Type+0x155C` (int) — `ProduceCashAmount` (INI: ProduceCashAmount)
- `Type+0x1560` (int) — `ProduceCashDelay` (INI: ProduceCashDelay, in frames)
- `Owner+0x34` → HouseTypeClass, `+0x1A6` — parasited/special flag

---

### Phase 7: Power Charge Bonus (from Overpowering)
**Address:** `0x0043FDA6`

```
13. if (this->field_0x294 != 0 && *(this->field_0x294 + 0x50) == this):
      this->vtable[0x124](2)  // SetMissionAndAnims(2) — triggers powered-up anim
```

**Fields:**
- `this+0x294` — pointer to some linked object (charge source)

---

### Phase 8: SAM/Gate Auto-Close Check
**Address:** `0x0043FDC0`

```
14. if (Type[0x16B8] != false):   // SAM=yes (from INI)
      if (this->field_0x2B4 != NULL):   // has transport/factory
        if (transport->vtable[0x54]() == false):  // transport is empty/not active
          this->vtable[0x3C8](0)  // BuildingClass::ToggleGate(0) — close gate
```

**Fields:**
- `Type+0x16B8` (bool) — `SAM` (INI key "SAM")
- `this+0x2B4` — Factory/transport pointer

---

### Phase 9: Building Animation State Machine Update
**Address:** `0x0043FDF0`

```
15. BuildingClass::UpdateAnimation(this)  // 0x004509D0
    - Updates frame counter, handles rate timer expiry
    - Manages turret/barrel facing direction
    - Updates active/idle/production/special anim slots based on:
      - Occupant state (garrison fire anims)
      - Radar dish rotation (if HasRadialIndicator)
      - Ore storage level (refinery/silo light anims)
      - SuperWeapon charge level (nuke silo staging anims)
      - Active power anim state (Production slot)
    - Checks if animation sequence completed → sets field_0x6DD = 1
    - Handles anim frame looping/wrap-around
```

---

### Phase 10: Mission Check — Clear Idle Flag
**Address:** `0x0043FE18`

```
16. if (this->vtable[0x200]() != false):    // checks if building has active mission
      if (this->field_0x534 != 0):          // has active anim
        if (this->vtable[0x1EC]() != false):  // MissionClass::Commence
          this->field_0x6DD = 0    // clear "anim complete" flag
```

---

### Phase 11: TechnoClass::AI_Update (Parent Class Update)
**Address:** `0x0043FE36`

```
17. TechnoClass::AI_Update(this)  // 0x006F9E50
    - Handles ALL inherited TechnoClass per-tick logic:
      a. Clear one-shot flag (field_0x431)
      b. Update turret anim looping sound
      c. Update temporal visual (chrono effect)
      d. Update gap visual
      e. Voice/sound queue processing (field_0x4F0)
      f. EMP stun countdown (field_0x298/0x29C)
      g. Health visual smoothing (field_0x70 → actual Health)
      h. Cloak/disguise sound handling (field_0xCA1)
      i. Veterancy rank change sound
      j. Thief/steal credits handling (per-tick drain, checks IsStealing)
      k. Parasite release check
      l. SelfHealing processing (vtable 0x298)
      m. Target validation (ally check, range check, dead check)
      n. TurretAnim spinning (frame based on ROT)
      o. Debris/spark particle system spawning when damaged
      p. Mission dispatch: MissionClass::Mission_Dispatch()
      q. Retaliation / threat scan timer (vtable 0x4C4, 0x4CC)
      r. Anim/movement warp check
      s. CaptureManager::Update (mind control)
      t. Self-regeneration (organic units — SelfHealing INI)
      u. Power plant wall healing (if PowersUnit type)
      v. Sensor update (vtable 0x4A0)
      w. Cloak proximity visibility checks
      x. Weapon range validation for current target
      y. Frame timer advancement
      z. EMP lock countdown → restore online effects
```

---

### Phase 12: Post-Parent Checks (Building-Specific)
**Address:** `0x0043FE3E`

```
18. if (this->field_0x90 == false):   // IsAlive / IsOnMap check
      return   // building was destroyed during TechnoClass::AI_Update

19. type = this->vtable[0x84]()   // GetTechnoType → BuildingTypeClass
    if (Type[0xCD5] != false && TechnoClass__GetGattlingValue() > 0):   // [corrected 2026-05-28: was Type->HasTurretAnim (0xCA1); binary uses Type[0xCD5] — verified via decompile_function 0x0043FB20, see '*(char *)(iVar3 + 0xcd5)']
      this->field_0x148 += 1   // Increment turret fire counter
```

---

### Phase 13: ROF / Reload Timer Check
**Address:** `0x0043FE6C`

```
20. if (this->field_0x2FC == 0 && Type[0x16C1] == false && Type[0x16C2] == false):
      // Not Hospital, not Armory — reset ROF from type
      this->field_0x2FC = Type+0x684   // ROF value from BuildingTypeClass
```

**Fields:**
- `Type+0x16C1` (bool) — `Hospital`
- `Type+0x16C2` (bool) — `Armory`
- `Type+0x684` (int) — ROF (Rate Of Fire) value
- `this+0x2FC` (int) — Current ROF timer

---

### Phase 14: Burst Fire Logic (Not in Mission_Guard)
**Address:** `0x0043FE90`

```
21. mission = this->vtable[0x184]()   // GetCurrentMission
    if (mission != 1):   // not Mission_Sleep
      if (Type[0xCD5] != false):   // [corrected 2026-05-28: was Type->HasTurretAnim (0xCA1); binary uses Type[0xCD5] — verified via decompile_function 0x0043FB20, ROOT_CAUSE: STRUCT_FAMILY_CASCADE]
        // Check burst fire timing
        elapsed_since_last_fire = g_CurrentFrameCounter - this->field_0x120
        if (elapsed_since_last_fire > RulesClass+0xE04 + 5):
          FUN_0070DE40(Type+0xD10)   // advance burst index
          currentBurst = TechnoClass__GetGattlingValue()
          maxBurst = FUN_0070DDC0()
          if (maxBurst > 0):
            if (IsElite):
              limit = Type + maxBurst*4 + 0xCF0  // EliteBurstDelay
            else:
              limit = Type + maxBurst*4 + 0xCD8  // BurstDelay
            if (currentBurst < limit):
              FUN_0070DDD0(maxBurst - 1)  // reset burst counter
      // Increment turret fire counter again
      if (Type[0xCD5] != false && TechnoClass__GetGattlingValue() > 0):   // [corrected 2026-05-28: same — binary uses Type[0xCD5] not Type[0xCA1]]
        this->field_0x148 += 1
```

**Fields:**
- `this+0x120` (int) — `LastFireFrame`
- `RulesClass+0xE04` (int) — `BurstDelay` base from rules
- `Type+0xCD8` (int[]) — `BurstDelay` array (normal)
- `Type+0xCF0` (int[]) — `BurstDelay` array (elite)
- `Type+0xD10` — Burst-related type data

---

### Phase 15: Clear Mission Idle Flag (Again)
**Address:** `0x0043FF2A`

```
22. Same check as Phase 10:
    if (vtable[0x200]() && vtable[0x1EC]()):
      this->field_0x6DD = 0
```

---

### Phase 16: Anim Slot State Change
**Address:** `0x0043FF4A`

```
23. pending_anim = this->field_0x538
    if (pending_anim != -1):
      if (this->field_0x534 != pending_anim):
        this->field_0x534 = pending_anim   // set new active anim state
        // Look up anim data from Type + pending_anim * 0xC + 0xF04 (anim table)
        first_frame = Type[pending_anim * 12 + 0xF04]
        num_frames = Type[pending_anim * 12 + 0xF08]
        rate = Type[pending_anim * 12 + 0xF0C]
        // For state 0 or 1, randomize rate
        if (pending_anim == 0 || pending_anim == 1):
          rate = FUN_005FB2E0(rate)   // randomized delay
        // Setup timer: start=CurrentFrame, duration=rate
        this->field_0x100 = g_CurrentFrameCounter
        this->field_0x108 = rate
        this->field_0x10C = rate
        this->field_0xF8 = first_frame
      this->field_0x538 = -1   // clear pending
```

**Fields:**
- `this+0x534` (int) — `CurrentAnimState` (active anim slot index)
- `this+0x538` (int) — `PendingAnimState` (-1 = none)
- `this+0xF8` (int) — `CurrentFrame` (current animation frame)
- `this+0xFC` (byte) — `FrameChanged` flag
- `this+0x100` (int) — `AnimTimer_StartFrame`
- `this+0x104` (int) — AnimTimer field 2
- `this+0x108` (int) — `AnimTimer_Duration`
- `this+0x10C` (int) — `AnimTimer_Rate`
- `this+0x110` (int) — Frame direction/step
- `Type+0xF04` — Building anim table (array of {first_frame, num_frames, rate}, 12 bytes each)

---

### Phase 17: Health Change → Sidebar Redraw
**Address:** `0x0043FFB8`

```
24. if (this->Health != this->field_0x544):   // health changed since last tick
      this->Owner[0x5778] = 1   // HouseClass needs sidebar update (power bar)
      this->Owner[0x5779] = 1   // HouseClass needs radar redraw
      this->field_0x544 = this->Health   // cache new health
```

**Fields:**
- `this+0x544` (int) — `LastTickHealth` (cached health from previous tick)
- `Owner+0x5778` (byte) — `HouseClass::NeedsSidebarUpdate`
- `Owner+0x5779` (byte) — `HouseClass::NeedsRadarUpdate`

---

### Phase 18: Zero Health → Destruction Sequence
**Address:** `0x0043FFE0`

```
25. if (this->Health == 0):
      // Remove all damage fire anims (same loop as Phase 2 removal)
      for i in 0..8:
        if (DamageFireAnims[i] != NULL):
          anim->Destroy()
          DamageFireAnims[i] = NULL

      // Check destruction delay timer (field_0x528/0x530)
      timer_remaining = this->field_0x530
      if (this->field_0x528 != -1):
        elapsed = g_CurrentFrameCounter - this->field_0x528
        if (elapsed >= timer_remaining): goto do_destroy
        timer_remaining -= elapsed

      if (timer_remaining != 0): return   // still waiting

      do_destroy:
      this->vtable[0xD4]()              // OnDestroyed callback
      BuildingClass::SpawnSurvivors(this)  // 0x00442D90
      this->vtable[0xF8]()              // ObjectClass::Limbo — remove from map
      BuildingClass::Place_OccupyMap()  // 0x00441F60 — update cell occupancy
      return
```

**Fields:**
- `this+0x528` (int) — `DestructionTimer_StartFrame` (-1 = not started)
- `this+0x530` (int) — `DestructionTimer_Duration`

---

### Phase 19: Delayed Fire Processing
**Address:** `0x00440074`

```
26. BuildingClass::ProcessDelayedFire(this)  // 0x004503F0
    - Handles queued fire commands (field_0x704 = DelayedFireType)
    - Decrements delay counter (field_0x714)
    - When counter reaches 0:
      - Type 1: Fire weapon at target (vtable 0x3C0), apply FirePower bonus
      - Type 2: Launch superweapon (FUN_0044ABD0)
    - Clears delayed fire state after execution
```

**Fields:**
- `this+0x704` (int) — `DelayedFireType` (0=none, 1=weapon, 2=superweapon)
- `this+0x708` (int) — `DelayedFireTarget`
- `this+0x70C` (int) — `DelayedFireParam2`
- `this+0x710` (int) — `DelayedFireParam3`
- `this+0x714` (int) — `DelayedFireCountdown`

---

### Phase 20: Overpowerable Unit List Cleanup
**Address:** `0x00440080`

```
27. if (Type[0x1575] != false):   // Overpowerable=yes
      // Clean up the overpowering unit list (field_0x670/0x67C)
      // Remove entries where the unit's building ptr != this
      for each unit in overpower_list (field_0x670 array):
        if (unit->field_0x2B4 != this):
          remove from list (compact array)

      // Set IsOverpowered flag based on count and power ratio
      count = this->field_0x67C
      if (count < 3):
        powerRatio = HouseClass::GetPowerRatio(Owner)
        if (powerRatio != 1.0 || !HasPower || count < 1):
          this->IsOverpowered = false
        else:
          this->IsOverpowered = true
      else:
        this->IsOverpowered = true
```

**Fields:**
- `Type+0x1575` (bool) — `Overpowerable` (INI key)
- `this+0x670` (int*) — Pointer to array of overpowering unit pointers
- `this+0x67C` (int) — Count of overpowering units
- `this->IsOverpowered` (bool) — Whether building is currently overpowered

---

### Phase 21: Auto-Sell / Civilian Building Capture Check
**Address:** `0x0044012C`

```
28. if (Type[0x157B] != false):   // field label unresolved
      BuildingClass::CheckAutoSellOrCivilian(this)  // 0x00458200
      - If Type+0x634 == -1 (no TechnoType reference / civilian building):
        - If health is Red → auto-sell the building
        - Find "Neutral" house (side matching building's original)
        - If occupant count == 0 AND owner != neutral → change owner to neutral
        - If occupant count > 0 AND owner == neutral → change owner to occupant's house
```

**Fields:**
- `Type+0x634` — TechnoType factory reference (-1 = civilian building)
- `Type+0x157B` (bool) — field label unresolved; this same byte is used above as
  the damage-fire threshold selector

---

### Phase 22: Repair + Power AI
**Address:** `0x0044013E`

```
29. BuildingClass::UpdateRepairAndPower(this)  // 0x00450630
    Handles two main subsystems:

    A. AI auto-sell/rebuild decision:
       - Only if HouseClass has enough credits (>= RulesClass+0x1444)
       - Only if not Selling/Construction mission
       - Only if building can accept upgrade (vtable 0x94)
       - Checks if house tech level < threshold (RulesClass+0x1758):
         YES → AI considers auto-selling damaged buildings
              (random chance based on Owner+0x1D0/0x1D4)
              Conditions: not in campaign, field_0x6DC set, repair toggle on,
              has enough money, not currently queued, not factory type 7,
              health < ConditionRed → queue sell (vtable 0x1A0 with param 1)
         NO → AI considers repairing
              If not already marked for repair:
                If field_0x6E8 set, or field_0x6E3 set, or field_0x6CB set,
                or IsPlayerControlled → start repair

    B. Repair tick processing (when field_0x6E8 is set):
       - Every N frames (based on repair interval from type cost):
         - Toggle repair wrench animation (field_0x6DE)
         - Calculate repair cost and HP increment from type
         - If house can't afford → stop repairing (field_0x6E8 = 0)
         - Else: spend money, add HP, check if fully repaired
         - If health crosses ConditionYellow threshold → update all anim slots
         - If health > ConditionYellow → remove red damage flash anim (field_0x310)
         - Redraw if frame changed
```

**Fields:**
- `this+0x6DC` (byte) — AI-controlled repair flag
- `this+0x6E3` (byte) — Repair queued flag
- `this+0x6CB` (byte) — Another repair trigger flag
- `this+0x6E8` (byte) — `IsBeingRepaired` (active repair state)
- `this+0x6DE` (byte) — Repair wrench animation toggle
- `this+0x3D1` (byte) — Repair order toggle
- `this+0x310` (AnimClass*) — Damage flash/spark anim
- `RulesClass+0x1444` (int) — Minimum credits for AI repair
- `RulesClass+0x1758` (int) — Tech level threshold for auto-sell vs repair decision

---

### Phase 23: Auto-Production (Factory Building Self-Production)
**Address:** `0x0044014E`

```
30. if (Type+0xEB8 != 0):   // Factory= type set (e.g., InfantryType, VehicleType, etc.)
      FUN_004500F0(this)   // BuildingClass auto-production handler

    This handles buildings that auto-produce units (like Cloning Vats clones):
    - If factory (field_0x524) exists and production is complete:
      - Check placement timer (field_0x550/0x558)
      - Try to exit/place the produced object (vtable 0x100)
        - Result 0: placement failed → abandon production, delete factory
        - Result 1: placement delayed → restart timer
        - Result 2: placement succeeded → play sound, record, complete production
    - If no factory exists but building type has Factory= and owner IsHuman:
      - If owner tech level >= 11:
        - Find primary factory building of same type
        - Create new FactoryClass, start production
        - Set production rate
```

**Fields:**
- `Type+0xEB8` (int) — `Factory` type enum (0=none, 1-8=factory types)
- `this+0x524` (FactoryClass*) — Active factory for auto-production
- `this+0x550` (int) — Production timer start frame
- `this+0x558` (int) — Production timer duration

---

### Phase 24: Bridge Destruction Timer
**Address:** `0x0044015C`

```
31. if (this->field_0x6DF != false):   // Bridge destruction pending
      // Check destruction timer (field_0x528/0x530)
      timer_remaining = this->field_0x530
      if (this->field_0x528 != -1):
        elapsed = g_CurrentFrameCounter - this->field_0x528
        if (elapsed >= timer_remaining): goto do_bridge_destroy
        timer_remaining -= elapsed

      if (timer_remaining != 0): goto Phase_25

      do_bridge_destroy:
      if (Type[0x16B6] == false):   // NOT BridgeRepairHut
        // Normal building destruction with damage
        this->vtable[0x16C](&health, 0, RulesClass+0xFA8, this->field_0x540, 1, 0, 0)
      else:
        // Bridge repair hut — actually destroy the bridge
        // Scan 5x5 area around building for bridge overlay tiles
        for x in -2..3:
          for y in -2..3:
            cell = this->vtable[0x1B8]()  // GetCell
            adjusted = cell + (x, y)
            check for bridge overlay types (DAT_00ABAD1C range)
            check for specific overlay IDs (0x4A..0x65)

        if (found high bridge):
          MapClass::DestroyBridge_High(cell)
        else:
          MapClass::DestroyBridge_Low(cell)

        this->field_0x6DF = 0
        this->field_0x540 = 0

      if (this->field_0x90 == false): return
      this->vtable[0x124](2)   // SetMissionAndAnims(2)
```

**Fields:**
- `this+0x6DF` (byte) — `BridgeDestructionPending`
- `this+0x540` (int) — Bridge destruction damage source
- `Type+0x16B6` (bool) — `BridgeRepairHut` (INI key)

---

### Phase 25: Factory/Transport Gate Logic — Final Check
**Address:** `0x00440378`

```
32. if (this->field_0x2B4 == 0): return   // no factory/transport

33. valid = this->vtable[0x3AC](this->field_0x2B4)  // validate transport
    if (valid): return

34. whatAmI = transport->vtable[0x2C]()   // WhatAmI
    if (whatAmI == 2):   // AircraftClass
      if (aircraft->vtable[0x50]() == false): return  // not ready
      this->vtable[0x3C8](0)   // ToggleGate(0)
      return

35. // For non-aircraft:
    this->vtable[0x3C8](0)   // ToggleGate(0) — close gate
    return
```

---

## Key Callee: BuildingClass::UpdateGapAndSpecialEffects (0x004549B0)

Called when power state changes (Phase 1). Handles all power-dependent special building effects.

### When building becomes POWERED (active_state = true):

1. **Robot Tank reactivation** — If `Type+0x40C` (UnitType for robots) != 0 and `field_0x662` was off:
   - Set `field_0x662 = 1`
   - Call `HouseClass::RobotTanksBackOnline(Type+0x40C)`

2. **Cloak Generator activation** — If `Type[0x16C7]` (CloakGenerator=yes):
   - If `field_0x6EB < 1` and `field_0x6EC != Type[0x1707]` (CloakRadiusInCells):
     - Set `field_0x6EB = 1` (expanding)
     - Mark dirty (`field_0x80 = 1`)

3. **Sensor Array activation** — If `Type[0xCD1]` (SensorArray related) and `field_0x269 == 0`:
   - Call `TechnoClass::UpdateCloakShroud()` (vtable 0x414)

4. **Powered building anim update** — If `Type[0x1573]` (Powered=yes) and `Type+0xEE4 > 0` (power drain):
   - Call `BuildingClass::OnPowerOff(this)` — updates anim slots for powered state

5. **PoweredSpecial anim cleanup** — If `Type[0x1574]` (PoweredSpecial=yes):
   - Clear and recreate specific anim slots based on health ratio
   - Iterates through 21 anim slots (stride 0x44, range 0..0x594)

### When building becomes UNPOWERED (active_state = false):

1. **Robot Tank deactivation** — Set `field_0x662 = 0`, call `HouseClass::RobotTanksOffline()`

2. **Mind control release** — If `field_0x2BC` (CaptureManager) != 0:
   - `CaptureManagerClass::FreeAll()` — releases all mind-controlled units

3. **Chrono warp abort** — If `field_0x2AC` != 0:
   - `BuildingClass::DeployUnit_ChronoWarp(1)` — cancel chronoshift

4. **Cloak Generator deactivation** — If `Type[0x16C7]` and `field_0x6EB >= 0` and `field_0x6EC != 0`:
   - Set `field_0x6EB = 0xFF` (contracting/off)
   - Mark dirty

5. **Sensor Array deactivation** — If `Type[0xCD1]` and `field_0x269 != 0`:
   - Call `TechnoClass::RemoveCloakShroud()` (vtable 0x418)
   - Clear extra power drain state
   - Update house power (`Owner[0x5778] = 1`)
   - Reset sensor range (`field_0x26C = Type[0xCD2]`)

6. **Powered anim update** — Same as powered path but calls `BuildingClass::OnPowerOn()`

7. **PoweredSpecial timer check** — If `Type[0x1574]`:
   - Check owner's PoweredSpecial timer (Owner+0x2A4/0x2AC)
   - Update anim slots based on health ratio and timer state

---

## Key Callee: BuildingClass::UpdateGapGenerator_Tick (0x00454DB0)

Called from vtable slot 260 (offset 0x410) during TechnoClass::AI_Update.

Manages the gap generator's shroud expansion/contraction animation per tick.

### State machine (`this+0x220` = GapState):

| State | Meaning |
|-------|---------|
| 0 | Off / inactive |
| 1 | Expanding (growing shroud) |
| 2 | Fully expanded (maintaining shroud) |
| 3 | Contracting (removing shroud) |

### Expansion (state 1):
- Increments `field_0x6ED` (current radius) by 1 per tick, up to 15
- At radii 1, 6, 11: marks dirty for redraw
- At radius 15: transitions to state 2 (fully expanded)
- Updates all 21 gap cells (`field_0x55C` array of 21 CellClass* pointers) with current radius
- Destroys particle system (field_0x30C) when fully expanded

### Contraction (state 3):
- Decrements `field_0x6ED` by 1 per tick, down to 0
- At radii 0, 5, 10: marks dirty for redraw
- At radius 0: transitions to state 0 (off)
- Creates idle particle system when fully contracted

### Shroud maintenance (states 2 and 0):
- State 2: If `ShouldUncloak` → call `vtable[0x45C](0)` (GapShroud expand)
- State 0: If `CanCloak` → call `vtable[0x460](0)` (GapShroud contract)

### Cloak generator radius propagation:
- If `field_0x6EB != 0` and `Type[0x16C7]` (CloakGenerator=yes):
  - Gets map size / 2 for radius calculation
  - **Expanding** (`field_0x6EB > 0`):
    - If current_radius == CloakRadiusInCells: set `field_0x6EB = 0` (done)
    - Else: increment `field_0x6EB`, apply cloak shroud via FUN_007BB920
  - **Contracting** (`field_0x6EB < 0`):
    - If current_radius == 0: set `field_0x6EB = 0` (done)
    - Else: decrement, remove cloak shroud
    - When fully contracted, check other gap generators in range and re-trigger them

**Fields:**
- `this+0x220` (int) — `GapState` (0=off, 1=expanding, 2=active, 3=contracting)
- `this+0x55C` — Array of 21 gap cell pointers (`field_0x55C` to `field_0x5B0`)
- `this+0x30C` (ParticleSystemClass*) — Gap generator idle particle
- `this+0x6EB` (signed byte) — Cloak generator radius expansion state (-1=contracting, 0=done, 1+=expanding)
- `this+0x6EC` (byte) — Current cloak radius
- `this+0x6ED` (signed byte) — Gap generator current shroud radius (0..15)
- `Type+0x16C7` (bool) — `CloakGenerator` (INI key)
- `Type+0x1707` (byte) — `CloakRadiusInCells` (INI key)

---

## Key Callee: BuildingClass::CreateDamageFireAnims (0x0043C0D0)

Creates fire animations on a damaged building.

- Gets `RulesClass+0x2B0` = number of fire anim types
- Picks random starting index into fire anim array (`RulesClass+0x2A4`)
- Iterates through building's fire positions (Type+0x15D8, stride 8, up to 8 positions)
  - Each position = isometric pixel offset {x, y}
  - Checks if position matches sentinel value (all zeros / disabled)
  - Converts isometric pixel to world coords
  - Adds building render offset
  - Creates AnimClass for the fire
  - Sets Z-adjust based on foundation size
  - Randomizes starting frame
- Fire anim pointers stored in `this+0x5C8..0x5E4` (array of 8)

**Fields:**
- `this+0x5C8..0x5E4` — 8x `AnimClass*` for damage fires
- `Type+0x15D8` — Array of 8 fire position {x,y} pairs (isometric pixels)
- `RulesClass+0x2A4` (int*) — Pointer to `DamageFireTypes` array
- `RulesClass+0x2B0` (int) — Number of fire anim types

---

## Key Callee: BuildingClass::ProcessDelayedFire (0x004503F0)

Handles queued weapon/superweapon fire commands.

```
if (this->DelayedFireType != 0):
  countdown = --this->DelayedFireCountdown
  if (countdown < 1):
    this->DelayedFireCountdown = 0
    if (DelayedFireType == 1):   // Weapon fire
      if (this->Target != 0):
        result = vtable[0x3C0](target, param2, 1)  // Fire weapon
        if (result == 0):
          result = vtable[0x3CC](target, param2)  // Alternate fire
          if (result != 0 && this->FirePowerBonus != 0):
            // Apply FirePower multiplier from IronCurtain/other buff
            modifier = (RulesClass+0x49C * FirePowerBonus + 100) * 256 / 100
            bullet->DamageMultiplier = modifier
            this->FirePowerBonus = 0
    else if (DelayedFireType == 2):  // Superweapon launch
      FUN_0044ABD0(param2, param3, param4)
    this->DelayedFireType = 0
```

**Fields:**
- `this+0x704` (int) — `DelayedFireType` (0=none, 1=weapon, 2=superweapon)
- `this+0x714` (int) — `DelayedFireCountdown` (frames remaining)
- `this+0x664` (int) — `FirePowerBonus` (from IronCurtain etc.)

---

## Key Callee: BuildingClass::UpdateAnimation (0x004509D0)

Large function (~1,600 bytes) managing all building animation state.

### Operations:
1. **Frame timer check** — If timer expired and rate != 0:
   - Advance frame (`field_0xF8 += field_0x110`)
   - Restart timer
   - Set `FrameChanged = 1`

2. **Turret facing** — Get current facing from cell, compute direction
   - `BuildingClass::UpdateAnimFacingAndDirection()`
   - `BuildingClass::SetAnimRemap()` based on translucency

3. **Garrison fire anim** — If `Type[0x16A9]` (CanC4=yes) and not Unloading mission:
   - If infantry in slots (field_0x57C, 0x588) and not IsOccupied(Type[0xCCE]):
     - Create garrison fire anim based on health (ActiveAnim/ActiveAnimDamaged)
     - Clear turret/barrel anims

4. **Radar dish rotation** — If `Type+0xEE8 > 0` (ExtraPower drain > 0) and `Type[0x16AF]`:
   - If placed on map and has active anim:
     - Get docking count → switch between Production/IdleAnim based on occupied state
     - Health-aware anim selection (normal vs damaged)

5. **Ore storage level anim** — If `Type[0x16A8]` (Weeder/refinery):
   - Calculate storage fraction (0..3) from `StorageClass::GetTotalAmount()`
   - Update anim slot 11 (SpecialAnim) visibility based on storage level
   - Set frame count on anim to match storage fraction

6. **Multi-level storage display** — If `Type[0x16BB]`:
   - Calculate storage quartile (0..3+)
   - Clear previous level anim, create new appropriate anim
   - Each level maps to different anim slots (Idle, Active, Production, Special)

7. **SuperWeapon charge staging anims** — If `Type+0x16F0 != -1`:
   - Not while Selling/Construction
   - Check `Type+0x16E8` (charge threshold float)
   - Find matching superweapon in owner's array
   - Calculate charge progress vs threshold
   - Switch between charge anim and ready anim based on progress

8. **Turret rotation anim** — If building has active turret anim:
   - Get current rate from RateTimer
   - Set turret anim frame based on shadow direction lookup table
   - Clear anim delay

9. **Anim completion handling** — When anim sequence reaches last frame:
   - Set `field_0x6DD = 1` (anim-complete flag)
   - If frame overflows → loop: restart with new randomized rate
   - Trigger `SetMissionAndAnims(2)` if looped

---

## BuildingClass Field Map Summary

### BuildingClass instance fields referenced in Update:

| Offset | Size | Name | Description |
|--------|------|------|-------------|
| 0x070 | 4 | VisualHealth | Smoothed health for display |
| 0x080 | 1 | NeedsRedraw | Dirty flag |
| 0x090 | 1 | IsOnMap | Whether building is placed on map |
| 0x0F8 | 4 | CurrentFrame | Current animation frame index |
| 0x0FC | 1 | FrameChanged | Set when frame advances |
| 0x100 | 4 | AnimTimer_Start | Frame counter when timer started |
| 0x104 | 4 | AnimTimer_Field2 | Timer helper field |
| 0x108 | 4 | AnimTimer_Duration | Timer duration in frames |
| 0x10C | 4 | AnimTimer_Rate | Rate of frame advancement |
| 0x110 | 4 | FrameStep | Frame increment direction |
| 0x120 | 4 | LastFireFrame | Frame counter of last weapon fire |
| 0x148 | 4 | TurretFireCounter | Incremented per burst tick |
| 0x220 | 4 | GapState | Gap generator state (0-3) |
| 0x269 | 1 | SensorActive | Sensor array active flag |
| 0x278 | 4 | DockedObject | Pointer to docked unit/aircraft |
| 0x294 | 4 | ChargeSource | Pointer to power charge object |
| 0x2AC | 4 | ChronoWarpRef | Chrono warp reference |
| 0x2B4 | 4 | FactoryOrTransport | Factory/transport pointer |
| 0x2BC | 4 | CaptureManager | Mind control manager pointer |
| 0x2FC | 4 | CurrentROF | Current Rate Of Fire timer |
| 0x310 | 4 | DamageSparkAnim | Damage spark AnimClass* |
| 0x4A0 | 4 | OccupantFrameCounter | IdleRate countdown for occupants |
| 0x524 | 4 | AutoFactory | Auto-production FactoryClass* |
| 0x528 | 4 | DeathTimer_Start | Destruction delay start frame |
| 0x530 | 4 | DeathTimer_Duration | Destruction delay duration |
| 0x534 | 4 | CurrentAnimState | Active building anim state index |
| 0x538 | 4 | PendingAnimState | Queued anim state (-1=none) |
| 0x540 | 4 | BridgeDamageSource | Bridge destruction warhead source |
| 0x544 | 4 | LastTickHealth | Cached health from previous tick |
| 0x550 | 4 | ProductionTimer_Start | Auto-production timer start |
| 0x558 | 4 | ProductionTimer_Duration | Auto-production timer length |
| 0x5C8 | 32 | DamageFireAnims[8] | 8x AnimClass* for damage fires |
| 0x5E8 | 1 | IsDamagedState | Cached damage fire state |
| 0x662 | 1 | RobotTanksOnline | Robot tank activation flag |
| 0x664 | 4 | FirePowerBonus | IronCurtain fire bonus multiplier |
| 0x670 | 4 | OverpowerList | Pointer to overpowering unit array |
| 0x67C | 4 | OverpowerCount | Number of overpowering units |
| 0x6B4 | ? | TransitionSound | Power transition sound data |
| 0x6C8 | 1 | LastActiveState | Cached operational state |
| 0x6CB | 1 | RepairTrigger3 | Repair trigger flag |
| 0x6D0 | 4 | ProduceCash_Start | ProduceCash timer start frame |
| 0x6D4 | 4 | ProduceCash_Field2 | ProduceCash timer field 2 |
| 0x6D8 | 4 | ProduceCash_Duration | ProduceCash timer remaining |
| 0x6DC | 1 | AIRepairFlag | AI-controlled repair flag |
| 0x6DD | 1 | AnimComplete | Animation sequence complete flag |
| 0x6DE | 1 | RepairWrenchToggle | Repair wrench anim toggle |
| 0x6DF | 1 | BridgeDestroyPending | Bridge destruction pending |
| 0x6E3 | 1 | RepairQueued | Repair queued flag |
| 0x6E8 | 1 | IsBeingRepaired | Active repair state |
| 0x6EB | 1 | CloakExpandState | Cloak gen radius state (signed) |
| 0x6EC | 1 | CloakCurrentRadius | Current cloak radius |
| 0x6ED | 1 | GapCurrentRadius | Gap gen current shroud radius |
| 0x6F0 | 4 | LastOreDisplayLevel | Cached ore display level |
| 0x704 | 4 | DelayedFireType | Queued fire type (0/1/2) |
| 0x714 | 4 | DelayedFireCountdown | Frames until fire executes |

### BuildingTypeClass fields referenced:

| Offset | Size | INI Key | Description |
|--------|------|---------|-------------|
| 0x40C | 4 | — | UnitType for robot tanks |
| 0x684 | 4 | ROF | Rate Of Fire |
| 0xCA1 | 1 | — | Occupant anim flag (gates OccupantFrameCounter in Phase 3 only) |
| 0xCD1 | 1 | — | SensorArray-related flag |
| 0xCD5 | 1 | — | Gattling/turret-fire flag — gates TurretFireCounter increments (Phases 12 and 14); distinct from 0xCA1 [corrected 2026-05-28: was missing; binary uses Type[0xCD5] for burst-fire gate — verified via decompile_function 0x0043FB20] |
| 0xCD2 | 1 | — | Sensor range value |
| 0xCD5 | 1 | — | Has looping sound |
| 0xCD8 | int[] | BurstDelay | Normal burst delay array |
| 0xCCE | 1 | — | Occupied building type flag |
| 0xCF0 | int[] | EliteBurstDelay | Elite burst delay array |
| 0xE80 | 4 | WorkingSound | Voc index for working sound |
| 0xE84 | 4 | NotWorkingSound | Voc index for not working sound |
| 0xEB8 | 4 | Factory | Factory type enum |
| 0xEE0 | 4 | Power | Power output (positive) |
| 0xEE4 | 4 | Power | Power drain (negative stored positive) |
| 0xEE8 | 4 | ExtraPower | Extra power output |
| 0xEEC | 4 | ExtraPower | Extra power drain |
| 0xF04 | varies | — | Anim table: array of {first_frame, num_frames, rate} × 12 bytes |
| 0x1558 | 4 | ProduceCashStartup | Initial cash grant on placement |
| 0x155C | 4 | ProduceCashAmount | Per-tick cash amount |
| 0x1560 | 4 | ProduceCashDelay | Frames between cash grants |
| 0x1573 | 1 | Powered | Requires power |
| 0x1574 | 1 | PoweredSpecial | Requires PoweredSpecial timer |
| 0x1575 | 1 | Overpowerable | Can be overpowered by units |
| 0x157B | 1 | label unresolved | Damage-fire threshold selector; also gates the auto-sell/civilian building check |
| 0x1580 | 4 | — | Warp anim position count |
| 0x15D8 | 64 | — | 8x fire position {x,y} pairs |
| 0x16A4 | 1 | Radar | Has radar functionality |
| 0x16A8 | 1 | — | Weeder / refinery with visible storage |
| 0x16A9 | 1 | — | CanC4 / garrison fire related |
| 0x16AF | 1 | — | Radar dish rotation flag |
| 0x16B6 | 1 | BridgeRepairHut | Is bridge repair hut |
| 0x16B8 | 1 | SAM | Is SAM site (auto-close gate) |
| 0x16BB | 1 | — | Multi-level ore display |
| 0x16C1 | 1 | Hospital | Is hospital |
| 0x16C2 | 1 | Armory | Is armory |
| 0x16C7 | 1 | CloakGenerator | Has cloak generator |
| 0x16C8 | 1 | SensorArray | Has sensor array |
| 0x16E8 | 4 | — | SuperWeapon charge threshold (float) |
| 0x16F0 | 4 | — | SuperWeapon type index |
| 0x1707 | 1 | CloakRadiusInCells | Cloak generator radius |

---

## TS-Legacy Notes

1. **Fog of War checks** — Several places in `UpdateGapAndSpecialEffects` and
   `UpdateGapGenerator_Tick` reference fog-of-war cell visibility. These code paths are
   active in YR (they drive the gap generator shroud system), but the general "fog of war"
   feature (`SpecialFlags & 0x1000`) defaults to OFF in YR skirmish. The gap generator
   shroud is a separate system that IS active.

2. **Robot Tank system** — The `HouseClass::RobotTanksBackOnline/Offline` calls in
   `UpdateGapAndSpecialEffects` are YR-active. Robot Tanks are a YR unit that depends
   on the Robot Control Center building being powered.

3. **Auto-production (FUN_004500F0)** — This system is active in YR for buildings with
   `Factory=` set and auto-production enabled (e.g., Cloning Vats clones). Not TS-legacy.

4. **Overpowerable** — Active in YR. Used by Soviet Battle Lab being overpowered by
   Tesla Troopers (YR-specific mechanic). The threshold of 3 units matches known YR behavior.

5. **BridgeRepairHut** — Active in YR. Used on bridge repair huts in maps.

6. **Power drain/output** — All power logic is YR-active. The `Powered`, `PoweredSpecial`
   system drives core building behavior.

---

## Complete Tick Order Summary

```
1.  Power state check (IsOperational)
2.  Looping sound update (WorkingSound / NotWorkingSound)
3.  Power state change effects (if changed):
    - Gap generator / cloak generator toggle
    - Sensor array toggle
    - Robot tank online/offline
    - Mind control release
    - Chrono warp abort
    - Anim slot updates
4.  Damage fire anim state check (create/remove fire anims)
5.  Occupant frame counter (IdleRate)
6.  Docked unit update
7.  Chrono warp check (early return if warping)
8.  ProduceCash timer (Oil Derrick money generation)
9.  Power charge bonus check
10. SAM gate auto-close check
11. Animation state machine update
12. Mission idle flag management
13. TechnoClass::AI_Update (parent class — mission dispatch, combat, etc.)
14. IsAlive check (return if destroyed)
15. Turret fire counter update
16. ROF timer reset (if not Hospital/Armory)
17. Burst fire logic
18. Anim slot state change (pending → active)
19. Health change → sidebar redraw
20. Zero health → destruction sequence (spawn survivors, limbo)
21. Delayed fire processing (weapon/superweapon)
22. Overpowerable unit list cleanup
23. Auto-sell / civilian building ownership check
24. Repair + Power AI
25. Auto-production (Factory building self-production)
26. Bridge destruction timer
27. Factory/transport gate logic
```
