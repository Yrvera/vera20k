# AnimClass Deep Dive -- Full Decompilation Report

Reverse-engineered from `gamemd.exe` via Ghidra MCP. Builds on `ANIM_CLASS_GHIDRA_REPORT.md`.
All offsets and behaviors verified from binary decompilation.

## AnimClass Virtual Function Table (0x7E3354)

124 entries total. AnimClass inherits ObjectClass which inherits AbstractClass.
`(I)` = inherited from ObjectClass/AbstractClass, `(O)` = overridden by AnimClass.

| VTable Index | Offset | Address | Name | Notes |
|:---:|:---:|:---:|---|---|
| 0 | 0x000 | 0x410260 | (I) AbstractClass::QueryInterface | |
| 1 | 0x004 | 0x410300 | (I) AbstractClass::AddRef | |
| 2 | 0x008 | 0x410310 | (I) AbstractClass::Release | |
| 3 | 0x00C | 0x426540 | (O) AnimClass::GetClassID | Returns CLSID for AnimClass |
| 4 | 0x010 | 0x410450 | (I) AbstractClass::IsDirty | |
| 5 | 0x014 | 0x425280 | (O) AnimClass::Load | Deserialization from save game |
| 6 | 0x018 | 0x4253B0 | (O) AnimClass::Save | Serialization to save game |
| 7 | 0x01C | 0x4103E0 | (I) AbstractClass::GetSizeMax | |
| 8 | 0x020 | 0x426590 | (O) AnimClass::GetRTTI | Returns RTTI type ID |
| 9 | 0x024 | 0x410470 | (I) AbstractClass::vfunc_9 | |
| 10 | 0x028 | 0x425150 | (O) AnimClass::Detach | Detach from dying/limbo objects |
| 11 | 0x02C | 0x426580 | (O) AnimClass::GetRefCount | |
| 12 | 0x030 | 0x426530 | (O) AnimClass::GetSize | Returns 0x1C8 |
| 13 | 0x034 | 0x425410 | (O) AnimClass::SaveExtras | Saves extra fields to stream |
| 14 | 0x038 | 0x410490 | (I) AbstractClass::GetOwnerIndex | |
| 15 | 0x03C | 0x4104A0 | (I) AbstractClass::vfunc_15 | |
| 16 | 0x040 | 0x4104B0 | (I) AbstractClass::vfunc_16 | |
| 17 | 0x044 | 0x5F6690 | (I) ObjectClass::vfunc_17 | |
| 18 | 0x048 | 0x422BE0 | (O) AnimClass::GetCoords | Returns coords + owner offset |
| 19 | 0x04C | 0x4104F0 | (I) AbstractClass::vfunc_19 | |
| 20 | 0x050 | 0x5F6B60 | (I) ObjectClass::vfunc_20 | |
| 21 | 0x054 | 0x5F6B90 | (I) ObjectClass::vfunc_21 | |
| 22 | 0x058 | 0x410540 | (I) AbstractClass::vfunc_22 | |
| 23 | 0x05C | 0x423AC0 | **(O) AnimClass::AI** | **Per-tick update** |
| 24 | 0x060 | 0x5F6DA0 | (I) ObjectClass::vfunc_24 | |
| 25 | 0x064 | 0x426390 | (O) AnimClass::IsAlive_stub | Returns false |
| 26 | 0x068 | 0x4263A0 | (O) AnimClass::vfunc_26 | Returns 0 |
| 27 | 0x06C | 0x5F3E30 | (I) ObjectClass::GetShape | Returns SHP pointer |
| 28 | 0x070 | 0x5F4250 | (I) ObjectClass::vfunc_28 | |
| 29 | 0x074 | 0x5F4240 | (I) ObjectClass::vfunc_29 | |
| 30 | 0x078 | 0x424CB0 | **(O) AnimClass::GetLayer** | Returns layer enum |
| 31 | 0x07C | 0x5F6C10 | (I) ObjectClass::vfunc_31 | |
| 32 | 0x080 | 0x4263B0 | (O) AnimClass::vfunc_32 | Returns false |
| 33 | 0x084 | 0x5F6BC0 | (I) ObjectClass::vfunc_33 | |
| 34 | 0x088 | 0x425520 | (O) AnimClass::Destroy | Self-removal + cleanup |
| 35 | 0x08C | 0x5F42A0 | (I) ObjectClass::vfunc_35 | |
| 36 | 0x090 | 0x4263C0 | (O) AnimClass::GetName | Returns L"No name" |
| 37 | 0x094 | 0x5F42B0 | (I) ObjectClass::vfunc_37 | |
| 38 | 0x098 | 0x5F42C0 | (I) ObjectClass::vfunc_38 | |
| 39 | 0x09C | 0x5F42D0 | (I) ObjectClass::vfunc_39 | |
| 40 | 0x0A0 | 0x5F42E0 | (I) ObjectClass::vfunc_40 | |
| 41 | 0x0A4 | 0x41BDD0 | (I) ObjectClass::vfunc_41 | |
| 42 | 0x0A8 | 0x5F6C80 | (I) ObjectClass::vfunc_42 | |
| 43 | 0x0AC | 0x41BE00 | (I) ObjectClass::GetRenderCoords | |
| 44 | 0x0B0 | 0x4263D0 | (O) AnimClass::GetCoords_Plus50 | Coords + Z offset 0x32 |
| 45 | 0x0B4 | 0x41BE30 | (I) ObjectClass::vfunc_45 | |
| 46 | 0x0B8 | 0x422BC0 | (O) AnimClass::GetRenderColor | Uses +0x184 or inherited |
| 47 | 0x0BC | 0x5F6A70 | (I) ObjectClass::vfunc_47 | |
| 48 | 0x0C0 | 0x426410 | (O) AnimClass::vfunc_48 | |
| 49 | 0x0C4 | 0x426420 | (O) AnimClass::vfunc_49 | |
| 50 | 0x0C8 | 0x426430 | (O) AnimClass::vfunc_50 | |
| 51 | 0x0CC | 0x41BE60 | (I) ObjectClass::vfunc_51 | |
| 52 | 0x0D0 | 0x41BE70 | (I) ObjectClass::vfunc_52 | |
| 53 | 0x0D4 | 0x425530 | (O) AnimClass::Limbo | Removes from cell |
| 54 | 0x0D8 | 0x5F4EC0 | (I) ObjectClass::Unlimbo | Places in world |
| 55 | 0x0DC | 0x5F5280 | (I) ObjectClass::vfunc_55 | |
| 56 | 0x0E0 | 0x5F42F0 | (I) ObjectClass::vfunc_56 | |
| 57 | 0x0E4 | 0x5F4300 | (I) ObjectClass::vfunc_57 | |
| 58 | 0x0E8 | 0x5F5940 | (I) ObjectClass::vfunc_58 | |
| 59 | 0x0EC | 0x5F4160 | (I) ObjectClass::vfunc_59 | |
| 60 | 0x0F0 | 0x426270 | (O) AnimClass::MarkCellOccupancy | Registers in cell |
| 61 | 0x0F4 | 0x426300 | (O) AnimClass::ClearCellOccupancy | Unregisters from cell |
| 62 | 0x0F8 | 0x4255B0 | **(O) AnimClass::Destroy** | **Cleanup and deletion** |
| 63 | 0x0FC | 0x5F4310 | (I) ObjectClass::vfunc_63 | |
| 64 | 0x100 | 0x5F4320 | (I) ObjectClass::vfunc_64 | |
| 65 | 0x104 | 0x422C70 | (O) AnimClass::GetActionOnObject | Action cursor |
| 66 | 0x108 | 0x4238D0 | (O) AnimClass::GetInvalidCoords | Returns sentinel coords |
| 67 | 0x10C | 0x426440 | (O) AnimClass::vfunc_67 | |
| 68 | 0x110 | 0x426450 | (O) AnimClass::vfunc_68 | |
| 69 | 0x114 | 0x422CA0 | **(O) AnimClass::DrawIt** | **Rendering** |
| 70 | 0x118 | 0x5F65D0 | (I) ObjectClass::vfunc_70 | |
| 71 | 0x11C | 0x5F4330 | (I) ObjectClass::vfunc_71 | |
| 72 | 0x120 | 0x5F4340 | (I) ObjectClass::vfunc_72 | |
| 73 | 0x124 | 0x4238B0 | (O) AnimClass::SetVisibility | Sets layer visibility |
| 74 | 0x128 | 0x5F4730 | (I) ObjectClass::vfunc_74 | |
| 75 | 0x12C | 0x5F4870 | (I) ObjectClass::vfunc_75 | |
| 76 | 0x130 | 0x41BE80 | (I) ObjectClass::vfunc_76 | |
| 77 | 0x134 | 0x5F4D10 | (I) ObjectClass::vfunc_77 | |
| 78 | 0x138 | 0x5F6C30 | (I) ObjectClass::vfunc_78 | |
| 79 | 0x13C | 0x5F6C70 | (I) ObjectClass::vfunc_79 | |
| 80 | 0x140 | 0x5F4360 | (I) ObjectClass::vfunc_80 | |
| 81 | 0x144 | 0x5F4350 | (I) ObjectClass::vfunc_81 | |
| 82 | 0x148 | 0x5F4370 | (I) ObjectClass::vfunc_82 | |
| 83 | 0x14C | 0x5F4520 | (I) ObjectClass::vfunc_83 | |
| 84 | 0x150 | 0x5F44A0 | (I) ObjectClass::vfunc_84 | |
| 85 | 0x154 | 0x426460 | (O) AnimClass::vfunc_85_noop | RET 0xC (no-op) |
| 86 | 0x158 | 0x426470 | (O) AnimClass::vfunc_86_noop | RET (no-op) |
| 87 | 0x15C | 0x426480 | (O) AnimClass::vfunc_87 | Returns 0 |
| 88 | 0x160 | 0x426490 | (O) AnimClass::vfunc_88 | Returns false |
| 89 | 0x164 | 0x5F4380 | (I) ObjectClass::vfunc_89 | |
| 90 | 0x168 | 0x5F4390 | (I) ObjectClass::vfunc_90 | |
| 91 | 0x16C | 0x5F5390 | (I) ObjectClass::vfunc_91 | |
| 92 | 0x170 | 0x4264A0 | (O) AnimClass::CanEnterCell | Stub: always returns 0 |
| 93 | 0x174 | 0x5F43A0 | (I) ObjectClass::vfunc_93 | |
| 94 | 0x178 | 0x5F43B0 | (I) ObjectClass::vfunc_94 | |
| 95 | 0x17C | 0x5F43C0 | (I) ObjectClass::vfunc_95 | |
| 96 | 0x180 | 0x5F43D0 | (I) ObjectClass::vfunc_96 | |
| 97 | 0x184 | 0x5F43E0 | (I) ObjectClass::vfunc_97 | |
| 98 | 0x188 | 0x41BE90 | (I) ObjectClass::vfunc_98 | |
| 99 | 0x18C | 0x4264B0 | (O) AnimClass::CanTarget | Stub: returns 0 |
| 100 | 0x190 | 0x5F5C20 | (I) ObjectClass::vfunc_100 | |
| 101 | 0x194 | 0x5F5930 | (I) ObjectClass::vfunc_101 | |
| 102 | 0x198 | 0x5F5930 | (I) ObjectClass::vfunc_102 | Same as 101 |
| 103 | 0x19C | 0x5F43F0 | (I) ObjectClass::vfunc_103 | |
| 104 | 0x1A0 | 0x5F4400 | (I) ObjectClass::vfunc_104 | |
| 105 | 0x1A4 | 0x5F6B50 | (I) ObjectClass::vfunc_105 | |
| 106 | 0x1A8 | 0x5F4410 | (I) ObjectClass::vfunc_106 | |
| 107 | 0x1AC | 0x4264C0 | (O) AnimClass::Receive_stub | XOR EAX,EAX; RET 0x14 |
| 108 | 0x1B0 | 0x4264D0 | (O) AnimClass::Click_stub | XOR EAX,EAX; RET 0x14 |
| 109 | 0x1B4 | 0x5F6940 | (I) ObjectClass::SetCoords | |
| 110 | 0x1B8 | 0x41BEA0 | (I) ObjectClass::vfunc_110 | |
| 111 | 0x1BC | 0x5F6960 | (I) ObjectClass::GetCell | |
| 112 | 0x1C0 | 0x5F69C0 | (I) ObjectClass::vfunc_112 | |
| 113 | 0x1C4 | 0x5F6A10 | (I) ObjectClass::vfunc_113 | |
| 114 | 0x1C8 | 0x5F5F40 | (I) ObjectClass::GetHeight | |
| 115 | 0x1CC | 0x5F5FA0 | (I) ObjectClass::SetHeight | |
| 116 | 0x1D0 | 0x425630 | (O) AnimClass::GetZAdjust | ZAdjust + owner's ZAdjust |
| 117 | 0x1D4 | 0x4264E0 | (O) AnimClass::vfunc_117 | |
| 118 | 0x1D8 | 0x4264F0 | (O) AnimClass::vfunc_118 | |
| 119 | 0x1DC | 0x426500 | (O) AnimClass::vfunc_119 | |
| 120 | 0x1E0 | 0x426510 | (O) AnimClass::vfunc_120 | |
| 121 | 0x1E4 | 0x426520 | (O) AnimClass::vfunc_121 | |
| 122 | 0x1E8 | 0x423930 | (O) AnimClass::ProcessBounceResult | Bounce physics handler |
| 123 | 0x1EC | 0x425510 | (O) AnimClass::GetEndFrame | Returns Type->End |

---

## 1. AnimClass::AI (0x423AC0) -- FULL DECOMPILATION

587 lines, 197 basic blocks, cyclomatic complexity 179. This is the heart of the animation system.

### Execution Flow (in order)

#### Phase 1: Special Behaviors (lines 1-100)

**a) PsiWarning check** (AnimType+0x373 `PsiWarning=yes`):
```
if (!this->field_0x198 && type->PsiWarning) {
    // When NOT flagged, make anim invisible
    this->IsInvisible = true;
}
// When flagged (Psi Warning active), make visible
// Controlled by global DAT_00a8eb7f (PsiWarning active flag)
```
If the type is the RulesClass PsiWarning anim (`g_RulesClass_Instance+0xB8`):
- If `DAT_00a8eb7f == 0` (no psi warning): `IsInvisible = 1`
- If `DAT_00a8eb7f != 0` (psi warning active): `IsInvisible = 0`

**b) HideIfNoOre check** (AnimType+0x359 `HideIfNoOre=yes`):
```
if (type->HideIfNoOre) {
    cell = GetCell();
    tiberiumValue = CellClass__Get_Tiberium_Value(cell);
    this->IsInvisible = (tiberiumValue == 0);
}
```
Hides the anim when the cell contains no tiberium.

**c) MakeInfantry check** (AnimType+0x34C `MakeInfantry`, != -1):
```
if (type->MakeInfantry != -1) {
    coords = this->Location;
    vtable->MarkCellOccupancy(coords); // Register position
}
```

**d) HasShadow tracking** (+0x11B):
```
if (this->field_0x11B && this->field_0x47 == this->CurrentFrame) {
    this->field_0x11B = false;
}
```

#### Phase 2: Bouncer Physics (lines 100-290)

**e) Bouncer/Meteor handling** (AnimClass+0x194 `IsBouncer`):
```
if (this->IsBouncer) {
    result = ProcessBounceResult();   // vtable[122] at 0x1E8 (NOT GetLayer)
    if (result == 1 || result == 2) { // 1=hit ground/bridge, 2=stopped
        // ... impact handling below
    }
}
```
(corrected 2026-05-29: was `GetLayer()` with layer-enum values; binary at vtable+0x1E8 calls
`AnimClass__ProcessBounceResult` which returns a BounceClass::Update() int result code —
1=hit-ground, 2=stopped — not a layer enum; verified via decompile_function 0x423AC0 +
read_memory vtable[0x1E8/4=slot 122]=0x00423930 + get_function_by_address 0x423930 —
RTTI_LABEL_DRIFT/OPERATOR_OR_ORDER_DRIFT)

When a bouncer hits the ground (`result == 1` means hit ground/bridge, `result == 2` means stopped):

1. **Check if over water or above ground**: Gets cell land type, checks ground height
2. **Meteor spawns** (IsMeteor check, AnimType+0x356):
   - If NOT over water AND NOT IsMeteor: spawn `g_RulesClass_Instance+0x94` (meteor impact anim) AND `g_RulesClass_Instance+0xBC4` (secondary impact)
   - If IsMeteor: spawn the last anim from the meteor array
   - If over water: different impact -- spawns the water impact anim
3. **Spawn child anims** (`Spawns`/`SpawnCount`): Randomly spawns `RandomRanged(0, SpawnCount) + RandomRanged(0, SpawnCount)` child anims at the impact position (corrected 2026-05-29: was `2 * RandomRanged(0, SpawnCount)`; binary does two independent RandomRanged(0,SpawnCount) calls and sums them — different distribution; verified via decompile_function 0x423AC0 — OPERATOR_OR_ORDER_DRIFT)
4. **Tiberium spreading** (IsTiberium && !above_ground):
   - Scans a radius of `TiberiumSpreadRadius` cells
   - For each valid cell in range: creates OverlayClass with TiberiumSpawnType
   - Random tiberium stage (0-2) assigned to each new cell
   - Tracks dirty screen rect for redraw
5. **Apply area damage**: If type has Warhead and DamageRadius, calls `Apply_area_damage`
6. **Self-destruct**: Calls `vtable->Destroy()`

#### Phase 3: Trailer Anim Spawning (lines 290-310)

```
if (this->IsActive && !this->IsInactive) {
    if (type->TrailerAnim != NULL && type->TrailerSeperation != 0) {
        if (TrailerSeperation == 1 || g_CurrentFrameCounter % TrailerSeperation == 0) {
            new AnimClass(type->TrailerAnim, GetCoords(), 0, 1, 0x600, 0, 0);
        }
    }
}
```
Spawns trailer anims periodically. `TrailerSeperation=1` means every frame.

#### Phase 4: Overlay/Veins Checking (lines 310-350)

If `type == g_RulesClass_Instance+0x147C` (a specific anim type, likely VEINHOLE):
- Looks up building in cell
- If building found: sets `IsInactive = 1`

If `type+0x360` flag is set (overlay checking):
- Gets coords, offsets by (-0x180, -0x180), checks cell for overlay
- If overlay doesn't match this type: sets `IsInactive = 1`

#### Phase 5: Frame Count / End Detection Setup (lines 350-365)

Auto-detects frame count from SHP if `End == -1`:
```
if (type->End == -1) {
    shp = type->GetShape();
    type->End = shp->NumFrames;   // from SHP header offset +6
    if (type->Shadow) {
        type->End /= 2;  // Shadow halves visible frames
    }
}
if (type->LoopEnd == -1) {  // offset 0x2BC (700 decimal)
    type->LoopEnd = type->End;
}
```

#### Phase 6: SetVisibility Call (lines 365-367)

Calls `vtable[0x124]` (SetVisibility) -- ensures display list is current.

#### Phase 7: Delay Countdown (lines 367-380)

```
// NOTE: field_0x19C one-shot check and Delay run BEFORE field_0x19E/Paused;
// field_0x19E/Paused gates only the CDTimer/frame-advance path below.
// Corrected 2026-05-29: was shown in wrong order (0x19E → Paused → 0x19C → Delay);
// binary order confirmed via decompile_function 0x423AC0 — OPERATOR_OR_ORDER_DRIFT.

if (this->field_0x19C) {        // +0x19C: one-shot flag (param_1[0x67])
    this->field_0x19C = 0;
    return;
}

if (this->Delay > 0) {
    this->Delay--;
    if (this->Delay == 0) {
        AnimClass::Middle();    // Begin playing!
    }
    return;
}
```

#### Phase 7b: CDTimer / Frame-Advance Paused Gates

```
if (this->field_0x19E) return;  // paused (+0x19E)
if (this->Paused) return;       // +0x11A
```
(These gates apply only to the CDTimer + frame-advance block below, not to the delay countdown.)

#### Phase 8: Frame Advancement (lines 380-410)

```
timerRemaining = CDTimerClass::GetTimeRemaining();
if (timerRemaining != 0 || this->FrameDelayReload == 0) {
    this->FrameAdvanced = false;
    return;
}
this->FrameAdvanced = true;
this->CurrentFrame += this->FrameStep;  // +1 or -1
this->LastFrameTime = g_CurrentFrameCounter;
this->field_0x2E = saved_value;
this->FrameDelay = this->FrameDelayReload;
```

#### Phase 9: Per-Frame Damage (lines 410-455)

```
if (type->Damage > 0.0 && !this->IsBouncer) {
    double multiplier = type->Damage;

    // If owner exists and owner is type 0x24 (36 = BuildingClass):
    //   multiplier *= 0.5  (DAT_007e3568 = 0.5)

    this->AccumulatedDamage += multiplier;

    if (this->AccumulatedDamage >= 1.0 && !this->field_0x198) {
        int damage = ftol(AccumulatedDamage);
        AccumulatedDamage -= damage;

        // Compare type name against "RING1" (0x8182F8)
        if (name == "RING1") {
            // Use C4Warhead (g_RulesClass_Instance+0xFA8) with no radius
            Apply_area_damage(coords, 0, C4Warhead, 1, 0);
        } else {
            // Use FlameDamage2 (g_RulesClass_Instance+0xF88)
            Apply_area_damage(coords, 0, FlameDamage2, 1, 0);
        }

        if (!this->IsActive) return;  // damage killed us
    }
}
```

The string comparison against "RING1" at 0x8182F8 is a byte-by-byte comparison of the
AnimTypeClass name (at type+0x24). This determines which warhead to use for damage.

#### Phase 10: Start Sound Trigger (lines 455-460)

```
if (type->SHP != NULL) {
    if (type->Start + this->CurrentFrame == type->SHP_frame_count) {
        if (!this->IsBouncer) {
            AnimClass::Start();  // Trigger start effects
        }
    }
}
```
Calls `Start()` when `CurrentFrame` reaches `SHP_total - Start` -- the midpoint.

#### Phase 11: PingPong Direction Reversal (lines 460-480)

```
if (type->PingPong) {
    if (LoopCountRemaining < 2) {
        if (CurrentFrame >= type->End || CurrentFrame == 0) {
            this->FrameStep = -this->FrameStep;  // Reverse direction
            return;
        }
    } else {
        if (CurrentFrame >= type->LoopEnd - type->Start || CurrentFrame == type->Start) {
            this->FrameStep = -this->FrameStep;
            return;
        }
    }
}
```

#### Phase 12: Loop / End / Next Transition (lines 480-587)

This is the most complex part. When `CurrentFrame >= End` (or `>= LoopEnd - Start` for looping):

**a) Loop count decrement:**
```
if (LoopCountRemaining != 0 && LoopCountRemaining != 0xFF) {
    LoopCountRemaining--;
}
```
`0xFF` = infinite looping (never decrements).

**b) If loops remaining (LoopCountRemaining > 0):**
```
if (type->Reverse || this->Reverse) {
    // Reverse: reset to LoopEnd
    CurrentFrame = type->LoopEnd;
} else {
    // Forward: reset to LoopStart - Start (adjusted)
    CurrentFrame = type->LoopStart - type->Start;
}

// Apply RandomLoopDelay if configured
if (type->RandomLoopDelay_Min != 0 || type->RandomLoopDelay_Max != 0) {
    this->Delay = RandomRanged(RandomLoopDelay_Min, RandomLoopDelay_Max);
}
return;  // Continue looping
```

**c) If no loops remaining -- check Next:**
```
AnimTypeClass* next = type->Next;  // type+0x2C8
if (next != NULL) {
    // MORPH the existing AnimClass -- no new allocation!
    this->Type = next;

    // Re-detect frame count if needed
    if (next->End == -1) {
        next->End = next->GetShape()->NumFrames;
        if (next->Shadow) next->End /= 2;
    }
    if (next->LoopEnd == -1) {
        next->LoopEnd = next->End;
    }

    // Reset all playback state
    this->IsInactive = false;
    this->LoopCountRemaining = next->LoopCount;
    this->AccumulatedDamage = 0.0;
    this->TranslucencyStage = 0;

    // Set up rate (with RandomRate support)
    rate = next->Rate;
    if (next->RandomRate_Min != 0 || next->RandomRate_Max != 0) {
        rate = RandomRanged(RandomRate_Min, RandomRate_Max);
    }

    // Handle Normalized rate
    if (next->Normalized) {
        rate = FUN_005fb2e0(rate);  // normalize
    }

    this->LastFrameTime = g_CurrentFrameCounter;
    this->FrameDelay = rate;
    this->FrameDelayReload = rate;
    this->CurrentFrame = next->Start;

    AnimClass::Middle();  // Begin playing the next anim
    return;
}
```

**KEY FINDING: Next does NOT create a new AnimClass.** It mutates the existing one in-place
by replacing the `Type` pointer and resetting playback state. Coordinates and owner attachment
are preserved. Next can chain indefinitely since it just keeps replacing Type.

**d) MakeInfantry spawn (no Next, end of animation):**
```
if (type->MakeInfantry != -1) {
    coords = this->Location;
    vtable->ClearCellOccupancy(coords);

    if (type->MakeInfantry <= g_RulesClass_Instance+0xCF4) {
        // Find appropriate owner house
        if (this->OwnerHouse == 0 || owner_is_observer) {
            country = GetCountryFromCell();
            // Search HouseClass array for matching country
            for each house:
                if house->Country == country:
                    this->OwnerHouse = house;
        }

        if (this->OwnerHouse != 0) {
            // Create aircraft from ParaDrop list
            AircraftTypeClass* type = AircraftTypeArray[...]
            aircraft = type->CreateInstance(this->OwnerHouse);

            // Try to place at coords
            if (!aircraft->CanPlace(coords, 0x60)) {
                this->CurrentFrame--;  // Retry next tick
                return;
            }

            // Handle cliff placement
            cell = GetCell(coords);
            if (cell is cliff) {
                ground_z = cell->GetCoords().Z;
                if (ground_z < this->Location.Z) {
                    aircraft->SetVisibility(false);
                    aircraft->IsActive = true;
                    aircraft->SetVisibility(true);
                }
            }

            // Mission assignment
            if (!owner_is_defeated) {
                aircraft->SetMission(0xF, 0);  // Mission 15 = Guard
            }
        }
    }

    this->IsMarkedForDeletion = true;
    vtable->Destroy();
    return;
}
```

**e) Simple end (no Next, no MakeInfantry):**
```
this->IsMarkedForDeletion = true;
vtable->Destroy();
```

---

## 2. AnimClass::DrawIt (0x422CA0) -- FULL DECOMPILATION

466 lines. Handles all rendering paths.

### Path 1: RING1 Expanding Ring (lines 55-150)

Only triggers when ALL conditions are met:
- `DAT_008a0df0 != 0` (Z-buffer rendering enabled globally)
- `g_ZBuffer != 0` (Z-buffer allocated)
- AnimType name matches "RING1" (string compare at 0x5F3E50)

**Ring size calculation:**
```c
int totalFrames = type->FrameDelayReload;  // [0x30]
int currentDelay = type->FrameDelay;       // [0x2F]

// If LastFrameTime is set, reduce currentDelay by elapsed time
if (this->LastFrameTime != -1) {
    int elapsed = g_CurrentFrameCounter - this->LastFrameTime;
    currentDelay = max(0, currentDelay - elapsed);
}

// Ring radius in pixels:
int ringSize = (totalFrames * CurrentFrame - currentDelay) + totalFrames;
// GetEndFrame() * totalFrames gives the max duration
int maxSize = GetEndFrame() * totalFrames;

int halfRing = ringSize + 8;  // padding
```

**Screen rect:**
```c
int left   = screenX - halfRing * 2;
int right  = screenX + halfRing * 2;
int top    = screenY - halfRing;
int bottom = screenY + halfRing;
```

**Alpha calculation (overall):**
```c
float remaining = (float)((maxSize - ringSize) * 256);
float alpha = remaining / maxSize;
alpha = clamp(alpha, 0, 255);
```

**Progressive alpha (two-pass):**

Pass 1 -- inner alpha (first 1/3 of duration):
```c
int oneThird = maxSize / 3;
if (ringSize < oneThird) {
    innerAlpha = (ringSize * 256) / oneThird;
} else {
    innerAlpha = remaining / (maxSize - oneThird);
}
innerAlpha = clamp(innerAlpha, 0, 255);
innerAlpha = (innerAlpha * 2) / 3;
```

Pass 2 -- outer alpha (first 2/3 of duration):
```c
int twoThirds = (maxSize * 2) / 3;
if (ringSize < twoThirds) {
    outerAlpha = (ringSize * 256) / twoThirds;
} else {
    outerAlpha = remaining / (maxSize - twoThirds);
}
outerAlpha = clamp(outerAlpha, 0, 255);
```

**Z-buffer integration:**
```c
int zBase = ((type->YDrawOffset + this->ZAdjust) - AdjustForZ())
            + (viewport_bottom - screenY) - 2;
float zFar  = (float)(zBase + halfRing) * Z_SCALE;
float zNear = (float)(zBase - halfRing) * Z_SCALE;
```

Draws 6 triangles (2 triangles = 1 quad) with `FUN_004a35f0` for vertex setup
and `FUN_004a3840` for triangle submission. Returns immediately after ring path.

### Path 2: Early Culling Checks (lines 150-165)

```c
if (framesSinceMapOpen > DAT_00abcd44 && type->DetailLevel > 1) return;
if (this->IsInvisible) return;
if (type->DetailLevel > DAT_00a8eb78) return;
if (this->field_0x199 && type->field_0x374) return;
```

### Path 3: Get SHP Data (lines 165-170)

```c
SHP* shp = vtable->GetShape();  // vtable[0x6C]
if (shp == NULL) return;
```

### Path 4: Frame Calculation (lines 170-175)

```c
int frameIndex = type->Start + this->CurrentFrame;
```

### Path 5: Translucency Flags (lines 175-240)

If `HasShadowOverride` (+0x119):
- `DoubleThick=no` -> flags |= 0x4 (50% translucent)
- `DoubleThick=yes` -> flags |= 0x6 (75% translucent)

If `type->Translucent` and detail level allows:
- **Fixed translucency** (`type->Translucency` is set):
  - 25 -> flags |= 0x2
  - 50 -> flags |= 0x4
  - 75 -> flags |= 0x6
- **Progressive translucency** (`type->Translucent=yes`, no explicit Translucency):
  - Uses `TranslucencyStage` (AnimClass+0x178, a char):
  - Stage 0-5: flags |= 0x2 (25%) (corrected 2026-05-29: Stage 0 was listed as "no translucency"; binary `cVar11 < '\x06'` includes Stage 0 in the 25% branch; verified via decompile_function 0x422CA0 — OPERATOR_OR_ORDER_DRIFT)
  - Stage 6-14: flags |= 0x4 (50%)
  - Stage 15+: return (fully invisible)
  - Based on fraction through total frames:
    - `dvar6 = (double)CurrentFrame`
    - `dvar7 = (double)type->End`
    - if `dvar6 > dvar7 * 0.75`: flags |= 0x6 (75%)
    - elif `dvar6 > dvar7 * 0.5`: flags |= 0x4 (50%)
    - elif `dvar6 > dvar7 * 0.25`: flags |= 0x2 (25%)

If bit 0 not set: add `0x800` (remap/key color flag).

### Path 6: Palette Selection (lines 240-300)

Three palette sources:
1. **IsVeins** (type+0x355): Uses player's color scheme palette `g_ColorSchemeArray[PlayerPtr->field_0x16054]`
2. **Has owner palette** (AnimClass+0xD4 nonzero): Uses stored palette directly, with `UseNormalLight` determining ground elevation lookup
3. **Default**: Gets palette from cell's theater palette or unit palette based on `UseNormalLight`

Height value for rendering: read from cell's `+0x10A` (surface height) or `+0x10C` (air height), depending on `UseNormalLight`.

### Path 7: Bouncer Draw (lines 300-315)

If `IsBouncer` (+0x194):
```c
int zAdj = AdjustForZ();
screenY += zAdj + type->YDrawOffset;
// Draw with shadow: flags = 0x2601
CC_Draw_Shape(shp, frame, screen, ..., 0x2601, ..., YDrawOffset - zAdj);
```
Draws the bouncer with shadow and ZAdjust compensation.

### Path 8: Building Tint (lines 315-375)

If `field_0x46` is set (anim attached to building):
- Look up building in cell
- If building has remap color: compute RGB565 tint from color scheme
- If building is powered down (status 1): compute different tint (powered-down color)
- Tint is passed to CC_Draw_Shape

### Path 9: Fog-of-War Check (lines 375-385)

Checks if cell is fogged (`FUN_00487950`). If fogged, zeroes out tint.

### Path 10: Standard Drawing (lines 385-420)

```c
if (!type->Tiled) {
    if (!type->Flat) {
        // NORMAL draw
        screenY += type->YDrawOffset;
        CC_Draw_Shape(shp, frame, screenPos, viewport, flags | 0x2000, 0,
                      (type->YDrawOffset + ZAdjust) - AdjustForZ() - 2,
                      palette, height, tint, ...);

        // SHADOW draw (if Shadow=yes)
        if (type->Shadow) {
            int shadowFrame = shp->NumFrames / 2 + frame;
            CC_Draw_Shape(shp, shadowFrame, screenPos, viewport,
                          (flags & ~0x6) | 0x601, 0,
                          -2 - AdjustForZ(), palette=1000, ...);
        }
    } else {
        // FLAT draw
        CC_Draw_Shape(shp, frame, screenPos, viewport, flags | 0x2000, 0,
                      (type->YDrawOffset + ZAdjust) - AdjustForZ() - 3, ...);
    }
}
```

### Path 11: Tiled Drawing (lines 420-466)

```c
if (type->Tiled) {
    int tileHeight = SHP_frame_rect_getter(0)->height;
    int y = screenY - tileHeight / 2;
    bool done = false;
    int zOffset = (type->YDrawOffset + ZAdjust) - AdjustForZ() - 0x32;

    do {
        CC_Draw_Shape(shp, frame, {screenX, y + YDrawOffset}, viewport,
                      flags | 0x2000, 0, zOffset, ...);
        if (y < 0) done = true;
        y -= tileHeight;
        zOffset -= (tileHeight/2 + tileHeight);
    } while (!done);
}
```
Tiles the animation vertically from bottom to top.

---

## 3. AnimClass::Middle (0x424CE0) -- FULL DECOMPILATION

66 lines. Called when delay countdown expires.

```c
void AnimClass::Middle() {
    // 1) Set visibility to layer 2
    vtable->SetVisibility(2);

    // 2) Play start sound
    if (!this->field_0x198 && type->StartSound != -1) {
        coords = vtable->GetCoords();
        PlaySoundAtCoords(type->StartSound, coords);  // FUN_007509e0
    } else {
        PlayDefaultSound();  // FUN_00405d40
    }
    PlayDefaultSound();  // Always called a second time

    // 3) If no SHP data, call Start() for effects
    if (type->SHP == NULL) {  // type+0x298 == 0
        AnimClass::Start();
    }

    // 4) Tiberium chain reaction
    if (!this->field_0x198 && type->TiberiumChainReaction) {
        cell = GetCell();
        tiberiumIndex = FUN_00485010(cell);  // Get tiberium overlay index

        if (tiberiumIndex != -1) {
            OverlayTypeClass* overlay = g_OverlayTypes[tiberiumIndex];

            // Reduce tiberium in this cell
            CellClass::Reduce_Tiberium(cell->tiberium_density + 1);

            // 33% chance to chain to adjacent cell
            if (overlay->ChainReactionDamage > 0) {
                if (Random() % 3 == 0) {
                    // Pick random neighbor
                    int neighbor = RandomRanged(0, overlay->ChainReactionDamage - 1);
                    AnimTypeClass* chainAnim = overlay->ChainReactionAnims[neighbor];
                    AnimClass* chain = new AnimClass(chainAnim, coords, 0, 1, 0x600, 0, 0);
                    chain->Palette = colorScheme->ConvertPalette;
                    chain->field_0xFC = cell->tiberium_height;
                }
            }

            // Apply C4 damage at the chain reaction point
            Apply_area_damage(coords, 0, g_RulesClass_Instance->C4Warhead, 0, 0);

            // Recalculate cell attributes and redraw
            CellClass::RecalcAttributes(cell);
            RedrawCell(cell->MapCoords);
        }
    }
}
```

---

## 4. AnimClass::Start (0x424F00) -- FULL DECOMPILATION

78 lines. Creates initial effects (scorch, crater, particles).

```c
void AnimClass::Start() {
    // 1) Get cell coords
    CoordStruct* coords = vtable->GetCoords();
    CellCoord cellCoord = {coords->X >> 8, coords->Y >> 8};
    CellClass* cell = MapClass::Get_CellClass(cellCoord);
    int debrisSize = 0x1E;  // default 30

    // 2) Get SHP dimensions if available
    SHP* shp = vtable->GetShape();
    if (shp != NULL) {
        if (type->SHP_Width == -1) {
            type->SHP_Width = SHP_frame_rect(type->SHP).width;
        }
        if (type->SHP_Height == -1) {
            type->SHP_Height = SHP_frame_rect(type->SHP).height;
        }
        debrisSize = type->SHP_Height;
    }

    // 3) Spawn particles (SpawnsParticle)
    if (type->SpawnsParticle != -1 && type->NumParticles > 0) {
        for (int i = 0; i < type->NumParticles; i++) {
            FUN_0062e430(g_ParticleSystemTypes[type->SpawnsParticle],
                         this->Location);
        }
    }

    // 4) Height check -- only create ground effects below 0x1E height
    int height = vtable->GetHeight();
    if (height >= 0x1E) return;

    // 5) Scorch marks (type->Scorch)
    if (type->Scorch) {
        if (type->Crater) {
            // Crater+Scorch: 50% chance to create
            int roll = RandomRanged(0, 0x7FFFFFFE);
            if (roll * probability < threshold) {
                SpawnDebris(coords, debrisSize);  // scorch mark
                return;
            }
        } else {
            SpawnDebris(coords, debrisSize);       // always create scorch
            return;
        }
    }

    // 6) Crater effects (type->Crater, without Scorch)
    if (type->Crater) {
        CellClass::Reduce_Tiberium(6);  // clear tiberium in crater

        if (type->ForceBigCraters) {
            Debris_Smoke(coords, 300, 1);   // big crater with smoke
        } else {
            Debris_Smoke(coords, debrisSize, 0);  // normal crater
        }
    }
}
```

---

## 5. AnimClass::Destroy (0x4255B0) -- FULL DECOMPILATION

27 lines. Clean, simple function.

```c
void AnimClass::Destroy() {
    // 1) Detach from owner's anim list
    if (this->OwnerObject != NULL) {
        OwnerObject->vtable->RemoveAnim(this);  // vtable offset 0x60
    }

    // 2) Clear owner reference
    AnimClass::SetOwnerObject(NULL);

    // 3) Stop any playing sound
    FUN_00406060();  // sound cleanup

    // 4) Play StopSound (if configured)
    // Corrected 2026-05-29: condition was `type->ExpireAnim != -1` at type+0x2fc;
    // type+0x2FC is StopSound (not ExpireAnim; ExpireAnim is at type+0x304);
    // verified via decompile_function 0x4255B0 + field map — OFFSET_RETYPED_WRONG.
    if (!this->field_0x198 && this->Type != NULL && type->StopSound != -1) {  // type+0x2FC
        CoordStruct* sparkleCoords = &this->SparkleCoords;  // +0x1B4
        coords = vtable->GetCoords(sparkleCoords);
        VocClass__PlayAt(type->StopSound, sparkleCoords);  // FUN_007509e0
    }

    // 5) Add to pending-delete list
    ObjectClass::UnInit();
}
```

**Note**: The ExpireAnim is NOT spawned as a new AnimClass here -- only the StopSound is played
through `FUN_007509e0`. The actual `ExpireAnim` spawning happens in the constructor's
fallthrough path (the constructor has a dual path: construction + destruction, reachable
when type == NULL or at end of life).

---

## 6. The "Next" Chaining System

### How it works (verified from AI, lines 480-530):

1. **No new AnimClass is created.** The existing instance mutates in-place.
2. `this->Type` pointer is replaced: `this->Type = type->Next;`
3. ALL playback state is reset:
   - `CurrentFrame = next->Start`
   - `LoopCountRemaining = next->LoopCount`
   - `AccumulatedDamage = 0.0`
   - `TranslucencyStage = 0`
   - `IsInactive = false`
4. Rate is recalculated (may use RandomRate if defined)
5. Frame count is auto-detected from SHP if needed
6. `AnimClass::Middle()` is called to begin the new anim
7. **Coordinates are preserved** -- same world position
8. **Owner attachment is preserved** -- still follows same object
9. **Can chain indefinitely** -- as long as each Next has a valid Next pointer
10. **Loop count does NOT reset from the original** -- uses new type's LoopCount

### Edge case: Next with MakeInfantry
If there is no Next AND `MakeInfantry != -1`, the infantry spawning logic runs
instead. If there IS a Next, MakeInfantry on the current type is ignored.

---

## 7. Attached Anim Behavior

### Position Following (AnimClass::GetCoords, vtable[18], 0x422BE0)

Position is computed EVERY time GetCoords is called (both AI and Draw):

```c
CoordStruct AnimClass::GetCoords() {
    if (this->OwnerObject != NULL) {
        CoordStruct myCoords = ObjectClass::GetCoords();   // base position offset
        CoordStruct ownerCoords = Owner->GetCoords();
        return {
            ownerCoords.X + myCoords.X,
            ownerCoords.Y + myCoords.Y,
            ownerCoords.Z + myCoords.Z
        };
    } else {
        return ObjectClass::GetCoords();
    }
}
```

The anim's own coordinates are treated as an **OFFSET** from the owner when attached.
This means:
- Moving units: anim follows automatically (recalculated every GetCoords call)
- The offset is set when `SetOwnerObject` is called
- Multiple anims on the same owner share the same base position

### Attachment (AnimClass::SetOwnerObject, 0x424B50)

```c
void AnimClass::SetOwnerObject(ObjectClass* newOwner) {
    // DETACH from old owner
    if (this->OwnerObject != NULL) {
        // Remove from display if shown
        if (this->InDisplayList) {
            RemoveFromDisplay();
        }

        // Check if any OTHER anim still references this owner
        bool otherAnimsShareOwner = false;
        for (int i = 0; i < g_AnimClass_Array_Count; i++) {
            AnimClass* other = g_AnimClass_Array[i];
            if (other != this && other->OwnerObject == this->OwnerObject) {
                otherAnimsShareOwner = true;
                break;
            }
        }

        // If no other anims reference the owner, clear owner's anim flag
        if (!otherAnimsShareOwner) {
            OwnerObject->vtable->ClearAnimFlag();
            OwnerObject->HasAnimAttached = false;
        }

        // Reset our coords to world position
        GetCoords(&tempCoords);
        this->OwnerObject = NULL;
        vtable->SetCoords(tempCoords);

        // Re-add to display
        if (wasInDisplay) {
            DisplayClass::Submit_Object(this);
        }
    }

    // ATTACH to new owner
    if (newOwner != NULL) {
        CoordStruct tempCoords;
        vtable->GetCoords(&tempCoords);
        RemoveFromDisplay();

        newOwner->HasAnimAttached = true;
        this->OwnerObject = newOwner;

        // Get owner's coords and set our position relative
        CoordStruct ownerCoords;
        newOwner->GetCoords(&ownerCoords);
        vtable->SetCoords(ownerCoords);  // Becomes the offset

        DisplayClass::Submit_Object(this);
    }
}
```

### Detachment (AnimClass::Detach, vtable[10], 0x425150)

Called when an object the anim references is being destroyed:

```c
void AnimClass::Detach(int objectPtr, int flags) {
    ObjectClass::Detach(objectPtr, flags);  // base class

    // Owner being destroyed
    if (this->OwnerObject == objectPtr && objectPtr != 0) {
        RemoveFromDisplay();
        Owner->vtable->RemoveAnim(this);
        this->OwnerObject = NULL;
        this->IsInactive = true;     // Stop playing
        vtable->SetVisibility(0);
    }

    // Type being destroyed (shouldn't happen normally)
    if (this->Type == objectPtr) {
        this->Type = NULL;
    }

    // Field 0x5F or 0x60 (OwnerHouse) being destroyed
    if (this->field_0x5F == objectPtr) {
        this->field_0x5F = NULL;
        vtable->Destroy();
    }
    if (this->OwnerHouse == objectPtr) {
        this->field_0x5F = NULL;
        vtable->Destroy();
    }
}
```

When the owner dies or enters limbo, the anim is marked inactive and stops playing.
It is NOT destroyed -- it just becomes invisible.

---

## 8. The Damage-Dealing System

### AnimTypeClass fields:
- `Damage` (double, +0x2A8): damage accumulated per frame advancement
- `DamageRadius` (int, +0x334): radius in cells for area damage
- `Warhead` (WarheadTypeClass*, +0x330): warhead for area damage

### Per-tick accumulation (in AI, Phase 9):

Damage is a **double** that accumulates fractionally:
1. Each frame tick: `AccumulatedDamage += type->Damage`
2. If owner is a BuildingClass (type 0x24): damage is halved (multiplied by 0.5)
3. When `AccumulatedDamage >= 1.0`: extract integer part, apply area damage
4. Remainder stays in accumulator for next tick

### Warhead selection:
- If anim name == "RING1": uses `g_RulesClass_Instance->C4Warhead` (+0xFA8)
- Otherwise: uses `g_RulesClass_Instance->FlameDamage2` (+0xF88)
- Note: This is independent of the type's own `Warhead` field, which is used
  for bouncer impact damage instead.

### Bouncer damage (in AI, Phase 2):
When a bouncer hits ground:
```c
Apply_area_damage(coords, 0, type->Warhead, 1, 0);
```
This uses the type's own `Warhead` and applies it at the impact point.

---

## 9. Cell Registration

### AnimClass::MarkCellOccupancy (vtable[60], 0x426270)

```c
void AnimClass::MarkCellOccupancy(CoordStruct* coords) {
    int subcell = CellClass::GetSubCell(coords);
    CellClass* cell = CellClass::Get_Cell_At(coords);
    int groundHeight = CellClass::GetGroundHeight(coords);

    if (groundHeight + BridgeHeight <= coords->Z
        && cell->flags_0x140 has BRIDGE bit) {
        // ON bridge: register in bridge occupancy
        cell->bridge_occupancy |= (1 << subcell);
        cell->bridge_anim_index = this->GetOwnerIndex();
    } else {
        // ON ground: register in ground occupancy
        cell->ground_occupancy |= (1 << subcell);
        cell->ground_anim_index = this->GetOwnerIndex();
    }
}
```

### AnimClass::ClearCellOccupancy (vtable[61], 0x426300)

Reverse of MarkCellOccupancy: clears the bit flag and sets index to -1 when
all subcell bits are cleared.

### Does an anim block movement or building?
The occupancy bits at cell+0x124 (ground) and cell+0x128 (bridge) are checked
with mask `0x1C` (bits 2,3,4 -- subcells 2-4). When these bits are clear,
the anim index is reset to -1. This suggests anims **can** block subcell
occupancy, similar to infantry, but only for the specific subcell they occupy.

### Scorch mark persistence
Scorch marks are created via `SpawnDebris()` which creates overlay objects
(OverlayClass). These are separate from AnimClass and persist in the cell's
overlay system after the animation ends. They are NOT AnimClass instances.

---

## 10. AnimClass::BounceAI (0x425670) -- Bouncer Movement

220 lines. Called every tick for bouncing animations (Bouncer=yes or IsMeteor=yes).
Handles position-following and movement physics.

### Two modes:

**a) Already at destination (coords match DAT_0089a178/7C/80 sentinel):**
- If not flagged as "arrived": Call `FindAttachTarget` to find a nearby valid cell
- If no valid cell or max retries (>6): mark as arrived, set frame to `RunningFrames*8+1`
- If valid cell: set destination, increment retry counter

**b) Moving toward destination:**
- Calculate direction vector: `destination - current_position`
- If distance < 0x13 (19 leptons): arrival check
  - If target cell is water or cliff: mark arrived
  - Otherwise: find new attachment target via `FindAttachTarget`
- If not arrived: calculate facing angle, compute movement
  - Uses sin/cos lookup for directional movement
  - Checks ground height to stay above terrain
  - Calls `vtable->SetCoords()` to update position
- Track frame based on direction and time: `CurrentFrame = (frameCounter/3) % RunningFrames + directionOffset`

### FindAttachTarget (0x425D10)
Searches a 10x10 grid around the current cell for a valid landing position:
- Must be passable terrain (land type == 2)
- Must not have bridge flag
- Adjacent cells must also be passable
- Returns closest valid cell by Manhattan distance

---

## AnimTypeClass Complete Field Map (param_1 is `int*`)

From ReadINI + Constructor, all verified:

| Byte Offset | Index | INI Key | Type | Default | Notes |
|:---:|:---:|---|---|:---:|---|
| 0x294 | 0xA5 | -- | int | -1 | Array index |
| 0x298 | 0xA6 | -- | SHP* | 0 | SHP data pointer |
| 0x29C | 0xA7 | -- | int | 0 | SHP width (cached) |
| 0x2A0 | 0xA8 | -- | int | 0 | SHP height (cached) |
| 0x2A8 | 0xAA-AB | Damage | double | 0.0 | Damage per frame |
| 0x2B0 | 0xAC | Rate | int | 1 | 900/INI_Rate ticks |
| 0x2B4 | 0xAD | Start | int | 0 | First frame |
| 0x2B8 | 0xAE | LoopStart | int | 0 | Loop restart frame |
| 0x2BC | 0xAF | LoopEnd | int | 0 | Loop end frame |
| 0x2C0 | 0xB0 | End | int | 0 | Total frames (-1=auto) |
| 0x2C4 | 0xB1 | LoopCount | int | 0 | Loop repetitions |
| 0x2C8 | 0xB2 | Next | AnimType* | 0 | Chain anim |
| 0x2CC | 0xB3 | SpawnsParticle | int | -1 | ParticleSystemType index |
| 0x2D0 | 0xB4 | NumParticles | int | 0 | Count |
| 0x2D4 | 0xB5 | DetailLevel | int | 0 | Min detail to show |
| 0x2D8 | 0xB6 | TranslucencyDetailLevel | int | 0 | Min detail for translucency |
| 0x2DC | 0xB7 | RandomLoopDelay (min) | int | 0 | |
| 0x2E0 | 0xB8 | RandomLoopDelay (max) | int | 0 | |
| 0x2E4 | 0xB9 | RandomRate (min) | int | 0 | 900/x converted |
| 0x2E8 | 0xBA | RandomRate (max) | int | 0 | 900/x converted |
| 0x2EC | 0xBB | Translucency | int | 0 | 25/50/75 |
| 0x2F0 | 0xBC | Spawns | AnimType* | 0 | Spawned child anims |
| 0x2F4 | 0xBD | SpawnCount | int | 0 | |
| 0x2F8 | 0xBE | StartSound | int | -1 | Sound index |
| 0x2FC | 0xBF | StopSound | int | -1 | Sound index |
| 0x300 | 0xC0 | BounceAnim | AnimType* | 0 | Impact anim |
| 0x304 | 0xC1 | ExpireAnim | AnimType* | 0 | End-of-life anim |
| 0x308 | 0xC2 | TrailerAnim | AnimType* | 0 | Periodic spawn |
| 0x30C | 0xC3 | TrailerSeperation | int | 0 | Frames between trailers |
| 0x310 | 0xC4-C5 | Elasticity | double | 0.8 | Bounce coefficient |
| 0x318 | 0xC6-C7 | MinZVel | double | 3.5 | Min Z velocity |
| 0x320 | 0xC8-C9 | MaxZVel | double | 3.5 | Max Z velocity |
| 0x328 | 0xCA-CB | MaxXYVel | double | 15.0 | Max XY velocity |
| 0x330 | 0xCC | Warhead | Warhead* | 0 | For bounce damage |
| 0x334 | 0xCD | DamageRadius | int | 0 | Cells |
| 0x338 | 0xCE | TiberiumSpawnType | Overlay* | 0 | |
| 0x33C | 0xCF | TiberiumSpreadRadius | int | 0 | Cells |
| 0x340 | 0xD0 | YSortAdjust | int | 0 | Sort order offset |
| 0x344 | 0xD1 | YDrawOffset | int | 0 | Y pixel offset |
| 0x348 | 0xD2 | ZAdjust | int | 0 | Z-order offset |
| 0x34C | 0xD3 | MakeInfantry | int | -1 | Infantry type |
| 0x350 | 0xD4 | RunningFrames | int | 0 | |
| 0x354 | 0xD5 | IsFlamingGuy | bool | false | |
| 0x355 | -- | IsVeins | bool | false | |
| 0x356 | -- | IsMeteor | bool | false | |
| 0x357 | -- | TiberiumChainReaction | bool | false | |
| 0x358 | 0xD6 | IsTiberium | bool | false | |
| 0x359 | -- | HideIfNoOre | bool | false | |
| 0x35A | -- | Bouncer | bool | false | |
| 0x35B | -- | Tiled | bool | false | |
| 0x35C | 0xD7 | ShouldUseCellDrawer | bool | true | |
| 0x35D | -- | UseNormalLight | bool | false | |
| 0x360 | -- | (OverlayCheck) | bool | false | |
| 0x361 | -- | AltPalette | bool | false | |
| 0x362 | -- | Normalized | bool | false | |
| 0x364 | 0xD9 | Layer | int | 3 | 3=Ground |
| 0x368 | 0xDA | DoubleThick | bool | false | |
| 0x369 | -- | Flat | bool | false | |
| 0x36A | -- | Translucent | bool | false | |
| 0x36B | -- | Scorch | bool | false | |
| 0x36C | 0xDB | Flamer | bool | false | |
| 0x36D | -- | Crater | bool | false | |
| 0x36E | -- | ForceBigCraters | bool | false | |
| 0x36F | -- | Sticky | bool | false | |
| 0x370 | 0xDC | PingPong | bool | false | |
| 0x371 | -- | Reverse | bool | false | |
| 0x372 | -- | Shadow | bool | false | |
| 0x373 | -- | PsiWarning | bool | false | |
| 0x374 | 0xDD | ShouldFogRemove | bool | true | |

---

## Bounce Physics (BounceClass::Update, 0x439B00)

233 lines. The full physics simulation for bouncing anims.

### State stored in BounceClass (embedded in AnimClass):

The bounce state is stored starting at AnimClass offset ~0x128 (indices 0x4A+).
Contains position, velocity, gravity, and elasticity as doubles/floats.

### Per-tick update:

1. **Apply gravity**: `velocity.Z -= gravity` (gravity stored in BounceClass)
2. **Apply max velocity cap**: If speed > `MaxXYVel`, normalize to cap
3. **Convert float position to int coords**: `ftol()` for X, Y, Z
4. **Get ground height at new position**: `CellClass::GetGroundHeight()`
5. **Bridge detection**: Check if passing through a bridge plane
6. **Ground collision**:
   - If new Z < ground height: check for bounce or impact
   - If bridge pass-through detected: snap to bridge surface
7. **Building collision**: Check if cell contains a building
   - If building is garrisonable and has 7+ occupants: pass through
   - If building is destroyed: pass through
8. **Bounce reflection**:
   - Record old velocity
   - Reflect velocity vector against surface normal
   - Apply elasticity: multiply reflected velocity by elasticity coefficient
   - Reverse rotation axes
9. **Slope deflection**: If crossing more than 1 height level change, deflect trajectory
10. **Return value**:
    - `0` = still flying (no collision)
    - `1` = hit ground/bridge (bounce or impact)
    - `2` = stopped (velocity below threshold)

### Return code handling (AnimClass::ProcessBounceResult, 0x423930):

```c
int result = BounceClass::Update();

if (type->IsMeteor) {
    this->field_0x155 += bounce_velocity;  // accumulate Z from velocity
}

if (result == 1) {  // Hit ground
    // Spawn BounceAnim if configured
    if (type->BounceAnim != NULL) {
        new AnimClass(type->BounceAnim, coords, 0, 1, 0x600, 0, 0);
    }

    // Apply area damage at impact
    if (type->Warhead != NULL) {
        Apply_area_damage(coords, type->DamageRadius, type->Warhead, ...);
    }

    // Scan units in impact cell and deal damage
    for (object in cell->ObjectList) {
        if (distance(object, impactCoords) <= type->DamageRadius) {
            object->ReceiveDamage(type->Warhead, damage);
        }
    }
} else if (result == 2) {  // Stopped
    vtable->Destroy();  // Remove the anim
}

// Update world position from bounce state
vtable->SetCoords(new_position_from_bounce);
```

---

## Functions Labeled in Ghidra This Session

| Address | Name |
|:---:|---|
| 0x422BE0 | AnimClass__GetCoords_WithOwnerOffset |
| 0x422BC0 | AnimClass__GetRenderColor |
| 0x422C70 | AnimClass__GetActionOnObject |
| 0x423930 | AnimClass__ProcessBounceResult |
| 0x4238D0 | AnimClass__GetInvalidCoords |
| 0x424CB0 | AnimClass__GetLayer |
| 0x425150 | AnimClass__Detach |
| 0x425280 | AnimClass__Load |
| 0x4253B0 | AnimClass__Save |
| 0x425410 | AnimClass__SaveExtras |
| 0x425510 | AnimClass__GetEndFrame |
| 0x425530 | AnimClass__Limbo |
| 0x425630 | AnimClass__GetZAdjust |
| 0x425670 | AnimClass__BounceAI |
| 0x425D10 | AnimClass__FindAttachTarget |
| 0x426270 | AnimClass__MarkCellOccupancy |
| 0x426300 | AnimClass__ClearCellOccupancy |
| 0x4264A0 | AnimClass__CanEnterCell_stub |
| 0x4264B0 | AnimClass__CanTarget_stub |
| 0x4264C0 | AnimClass__Receive_stub |
| 0x4264D0 | AnimClass__Click_stub |
| 0x426460 | AnimClass__vfunc_stub_ret |
| 0x426470 | AnimClass__vfunc_noop_stub |
| 0x426480 | AnimClass__vfunc_ret0_stub_85 |
| 0x426490 | AnimClass__vfunc_ret_false_stub |
| 0x4265B0 | AnimClass__CalcFacingDir |
| 0x439B00 | BounceClass__Update |
| 0x4399A0 | CoordStruct__FromDoubles |
