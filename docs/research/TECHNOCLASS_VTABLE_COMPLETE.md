# TechnoClass Complete Vtable Map

> **[CORRECTED 2026-05-19]** vtable slot +0x484 is mislabeled here as `GetTarget_484` ("Get current target"). The slot is actually **post-arrival mission dispatch** (`OnArrival` → convoy dequeue → `Queue_Mission`), base implementation at `0x00709A40`, UnitClass override at `0x00738970` (Scatter_Force), InfantryClass override at `0x0051CBA0` (IdleDispatch). Called from Drive::Process arrival paths only when `FootClass+0x598 != 0` (waypoint queue non-empty). See `TECHNO_VTABLE_0x484_DRIVE_PROCESS_ARRIVAL_GHIDRA_REPORT.md` for the corrected analysis.

**Source:** Ghidra decompilation of gamemd.exe  
**Vtable address:** `0x007F4960` (primary), `0x007F4944` (IFoo +4), `0x007F493C` (IFoo +8), `0x007F4934` (IFoo +12)  
**Total entries:** 309 (indices 0-308), spanning offsets `0x000`-`0x4D0`  
**Confidence:** High - addresses read directly from vtable memory, function names from Ghidra labels + decompilation.

## Inheritance Hierarchy

```
AbstractClass          (entries ~0-22)
  ObjectClass          (entries ~23-68)
    MissionClass       (entries ~69-156, includes mission handlers)
      RadioClass       (entries ~157-160)
        TechnoClass    (entries ~161-308)
```

## Class Boundary Notes

The vtable boundaries are approximate because subclasses override parent entries freely.
Entries 129-156 are MissionClass mission handler slots (Mission_Sleep through Mission_Patrol).
Entries 157-160 are RadioClass radio transmission virtuals.
Entries 161+ are TechnoClass-specific methods.

---

## Complete Vtable

### IPersistStream / IUnknown Interface (entries 0-7)

| Idx | Offset | Address    | Name | Description |
|-----|--------|------------|------|-------------|
| 0   | 0x000  | 0x00410260 | AbstractClass::QueryInterface | COM IUnknown - query interface |
| 1   | 0x004  | 0x00410300 | AbstractClass::AddRef | COM IUnknown - add reference |
| 2   | 0x008  | 0x00410310 | AbstractClass::Release | COM IUnknown - release reference |
| 3   | 0x00C  | 0x004C9150 | Stub::ReturnZero | IPersistStream::GetClassID (stub) |
| 4   | 0x010  | 0x00410480 | Stub::ReturnVoid_010 | IPersistStream::IsDirty (stub — corrected 2026-05-28: was 0x00410450 AbstractClass__IsDirty; binary vtable at 0x007F4970 holds 0x00410480 FUN_00410480 = 2-byte void stub; AbstractClass__IsDirty confirmed at 0x00410450 but NOT in this slot — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 5   | 0x014  | 0x0070BF50 | TechnoClass::Load | IPersistStream::Load - deserialize from stream, registers pointers with swizzle manager |
| 6   | 0x018  | 0x0070C250 | TechnoClass::Save_Stream | IPersistStream::Save - delegates to RadioClass save |
| 7   | 0x01C  | 0x004103E0 | AbstractClass::GetSizeMax | IPersistStream::GetSizeMax |

### AbstractClass Methods (entries 8-22)

| Idx | Offset | Address    | Name | Description |
|-----|--------|------------|------|-------------|
| 8   | 0x020  | 0x007106E0 | TechnoClass::ScalarDeletingDestructor | Destructor (calls ~MissionClass, optionally frees memory) |
| 9   | 0x024  | 0x006F3F40 | TechnoClass::Init_Managers | Initializes flash, cloak, gap-gen, and other subsystem timers |
| 10  | 0x028  | 0x007077C0 | TechnoClass::PointerExpired | Nullify dangling pointers when objects are destroyed |
| 11  | 0x02C  | 0x004C9150 | Stub::ReturnZero | AbstractClass::WhatAmI (pure virtual, overridden by subclass) |
| 12  | 0x030  | 0x004C9150 | Stub::ReturnZero | AbstractClass::SizeOf (pure virtual, overridden by subclass) |
| 13  | 0x034  | 0x0070C270 | TechnoClass::Save | Compute/save checksum and marshal all TechnoClass fields |
| 14  | 0x038  | 0x006F9DB0 | GetOwnerHouseID | Returns owner house index (this+0x21C) |
| 15  | 0x03C  | 0x006F9DC0 | GetOwnerHousePtr | Returns owner house pointer (this+0x21C) |
| 16  | 0x040  | 0x004104B0 | ReturnFalse | AbstractClass stub returning 0 |
| 17  | 0x044  | 0x005F6690 | ObjectClass::IsDead | Checks health <= 0 (limbo + dead flags) |
| 18  | 0x048  | 0x005F65A0 | ObjectClass::GetCoords | Returns 3D coordinates (X,Y,Z) |
| 19  | 0x04C  | 0x004104F0 | GetFacingCoords | Returns coords (delegates to GetCoords) |
| 20  | 0x050  | 0x005F6B60 | ObjectClass::IsLowFlying | Checks if low-flying based on type flags |
| 21  | 0x054  | 0x005F6B90 | ObjectClass::IsHighFlying | Checks if high-flying based on type flags |
| 22  | 0x058  | 0x00410540 | GetTargetCoords | Returns target coordinates (delegates to GetCoords) |

### ObjectClass Methods (entries 23-68)

| Idx | Offset | Address    | Name | Description |
|-----|--------|------------|------|-------------|
| 23  | 0x05C  | 0x006F9E50 | TechnoClass::AI_Update | Main per-tick AI update - handles cloaking, temporal, spawners, etc. |
| 24  | 0x060  | 0x00710410 | TechnoClass::Detach | Detach from mind-control/temporal/etc. when target removed |
| 25  | 0x064  | 0x006F32D0 | TechnoClass::ReadINI | Read object state from INI (map loading) |
| 26  | 0x068  | 0x00703860 | TechnoClass::GetVisualState | Get current visual/animation state for rendering |
| 27  | 0x06C  | 0x005F3E30 | ObjectClass::ObjectType_GetAction | Gets action cursor from ObjectType |
| 28  | 0x070  | 0x00700600 | TechnoClass::What_Action_OnCell | Determine cursor/action for cell (move, attack, deploy, etc.) |
| 29  | 0x074  | 0x006FFEC0 | TechnoClass::What_Action_OnObject | Determine cursor/action when hovering over another object |
| 30  | 0x078  | 0x005F4260 | ObjectClass::GetThreatLevel | Returns threat value (checks IsAlive, compares to RulesClass threshold) |
| 31  | 0x07C  | 0x005F6C10 | ObjectClass::IsAboveGround | Is this object above ground level |
| 32  | 0x080  | 0x004263B0 | ObjectClass::GetShapeFilename | Returns shape filename for rendering |
| 33  | 0x084  | 0x006F3270 | TechnoClass::GetTechnoType | Returns pointer to TechnoTypeClass |
| 34  | 0x088  | 0x004E0130 | ObjectClass::GetObjectType | Returns pointer to ObjectTypeClass |
| 35  | 0x08C  | 0x00708B30 | TechnoClass::GetTimerStruct | Returns timer/rate data structure |
| 36  | 0x090  | 0x004263C0 | GetPixelSelectionBracketDelta | Pixel offset delta for selection brackets |
| 37  | 0x094  | 0x00701140 | TechnoClass::IsDeployable | Can this unit deploy? Checks deploysInto/type flags |
| 38  | 0x098  | 0x005F42C0 | ObjectClass::ReturnFalse_098 | Stub returning false |
| 39  | 0x09C  | 0x007010D0 | TechnoClass::CanDeploy | Checks if unit can currently deploy (player, weapon, etc.) |
| 40  | 0x0A0  | 0x00700C40 | TechnoClass::CanMove | Checks if unit can currently move (EMP, tethered, deploying, etc.) |
| 41  | 0x0A4  | 0x0041BDD0 | GetSomeCoords_0A4 | Returns coordinates (delegates to GetCoords) |
| 42  | 0x0A8  | 0x005F6C80 | ObjectClass::GetExitCoords | Returns exit/spawn coordinates |
| 43  | 0x0AC  | 0x0041BE00 | ObjectClass::GetRenderCoords | Returns coordinates used for rendering |
| 44  | 0x0B0  | 0x006F3AD0 | TechnoClass::GetFLH | Get Firing/Launch/Height offset for weapon hardpoint |
| 45  | 0x0B4  | 0x0041BE30 | GetCoords_0B4 | Returns firing offset coords |
| 46  | 0x0B8  | 0x005F6BD0 | ObjectClass::GetYSort | Get Y-sorting priority for draw order |
| 47  | 0x0BC  | 0x005F6A70 | ObjectClass::ShouldBeOnBridge | Check if object should render at bridge level |
| 48  | 0x0C0  | 0x00426410 | ObjectClass::GetFoundation | Returns foundation footprint |
| 49  | 0x0C4  | 0x0041C010 | ReturnFalse_0C4 | Stub |
| 50  | 0x0C8  | 0x0041C020 | ReturnFalse_0C8 | Stub |
| 51  | 0x0CC  | 0x0041BE60 | ReturnFalse_0CC | Stub |
| 52  | 0x0D0  | 0x0041BE70 | ReturnFalse_0D0 | Stub |
| 53  | 0x0D4  | 0x006F6AC0 | TechnoClass::Limbo | Put into limbo (remove from map, release sensors/sounds/etc.) |
| 54  | 0x0D8  | 0x006F6CA0 | TechnoClass::Unlimbo | Place on map (add sensors, reveal, etc.) |
| 55  | 0x0DC  | 0x005F5280 | ObjectClass::Destroy | Handle destruction sequence |
| 56  | 0x0E0  | 0x00702D40 | TechnoClass::RecordKill | Record kill credit (veterancy, score tracking) |
| 57  | 0x0E4  | 0x00703230 | TechnoClass::KillPassengers | Kill/eject passengers on destruction |
| 58  | 0x0E8  | 0x005F5940 | ObjectClass::Unlimbo | Base Unlimbo implementation |
| 59  | 0x0EC  | 0x005F4160 | ObjectClass::DropIn | Drop unit into map at location |
| 60  | 0x0F0  | 0x005F60A0 | ObjectClass::Mark_Put | Mark cells as occupied when placing |
| 61  | 0x0F4  | 0x005F6120 | ObjectClass::Mark_Remove | Unmark cells when removing |
| 62  | 0x0F8  | 0x005F65F0 | ObjectClass::UnInit | Final cleanup on object removal |
| 63  | 0x0FC  | 0x00703850 | TechnoClass::UpdateSensors_0FC | Calls UpdateReveal(0) to refresh sensor coverage |
| 64  | 0x100  | 0x007099D0 | TechnoClass::DrawPipScalePips | Draw tethered/ammo/storage pip scale indicators |
| 65  | 0x104  | 0x005F4B10 | ObjectClass::DrawIt | Base draw function |
| 66  | 0x108  | 0x005F5B90 | ObjectClass::DrawVoxelShadow | Draw shadow for voxel-based objects |
| 67  | 0x10C  | 0x006F60D0 | TechnoClass::DrawBehind | Draw elements that render behind the main sprite |
| 68  | 0x110  | 0x006F5190 | TechnoClass::DrawExtras | Draw selection brackets, health bar, pips, lines |

### MissionClass / Notification Methods (entries 69-128)

| Idx | Offset | Address    | Name | Description |
|-----|--------|------------|------|-------------|
| 69  | 0x114  | 0x005B3A50 | MissionClass::Mission_Load_Notify | Called when loading completes |
| 70  | 0x118  | 0x005F65D0 | TechnoClass::DrawVeterancyPips | Draw veteran/elite rank indicators |
| 71  | 0x11C  | 0x006F4A40 | TechnoClass::CheckPlayerDiscovery | Check if player discovers this hidden unit |
| 72  | 0x120  | 0x0070ADC0 | TechnoClass::UpdateSensors | Add/update sensor map coverage |
| 73  | 0x124  | 0x006F4A70 | TechnoClass::ProcessCloakAndNotify | Process cloak state transitions and discovery |
| 74  | 0x128  | 0x005F4730 | ObjectClass::GetDrawExtent | Get rendering extent rectangle |
| 75  | 0x12C  | 0x005F4870 | ObjectClass::GetDrawRect | Get actual draw rectangle |
| 76  | 0x130  | 0x0041BE80 | ReturnFalse_130 | Stub |
| 77  | 0x134  | 0x005F4D10 | ObjectClass::MarkNeedsRedraw | Mark object for redraw |
| 78  | 0x138  | 0x005F6C30 | ObjectClass::CanBeSelected | Check if object is selectable |
| 79  | 0x13C  | 0x006FC030 | TechnoClass::CanBeSelectedNow | Check if currently selectable (not cloaked, not deploying, etc.) |
| 80  | 0x140  | 0x005F4360 | ObjectClass::ClickedAction_140 | Handle click action |
| 81  | 0x144  | 0x005F4350 | ObjectClass::ClickedAction_144 | Handle click action |
| 82  | 0x148  | 0x006F9DD0 | TechnoClass::SetOwnerHouse | Set owning house (with discovery check) |
| 83  | 0x14C  | 0x006FBFA0 | TechnoClass::Select | Select this unit (play voice, etc.) |
| 84  | 0x150  | 0x005F44A0 | ObjectClass::Deselect | Deselect this unit |
| 85  | 0x154  | 0x0070E2B0 | TechnoClass::IronCurtain | Apply Iron Curtain invulnerability |
| 86  | 0x158  | 0x0070E340 | SetCoords_158 | Set coordinates (with extra processing) |
| 87  | 0x15C  | 0x0070E300 | SetCoords_15C | Set coordinates variant |
| 88  | 0x160  | 0x0041BF40 | TechnoClass::IsIronCurtainActive | Check if Iron Curtain is currently active |
| 89  | 0x164  | 0x006F7970 | TechnoClass::InRange | Check if target is within weapon range |
| 90  | 0x168  | 0x007012C0 | TechnoClass::GetWeaponRange | Get effective weapon range (considers spawned aircraft range) |
| 91  | 0x16C  | 0x00701900 | TechnoClass::ReceiveDamage | Process incoming damage (armor, warhead, veterancy, etc.) |
| 92  | 0x170  | 0x00710460 | TechnoClass::FreeAllMindControlCaptures | Release all mind-controlled units |
| 93  | 0x174  | 0x005F43A0 | ObjectClass::Scatter_174 | Scatter away from threat |
| 94  | 0x178  | 0x005F43B0 | ObjectClass::Scatter_178 | Scatter variant |
| 95  | 0x17C  | 0x005F43C0 | ObjectClass::Scatter_17C | Scatter variant |
| 96  | 0x180  | 0x00707DD0 | TechnoClass::GetThreatScore | Calculate threat value (cost + passengers + power) |
| 97  | 0x184  | 0x005B3040 | MissionClass::GetCurrentMission | Returns current mission enum |
| 98  | 0x188  | 0x0041BE90 | ReturnFalse_188 | Stub |
| 99  | 0x18C  | 0x006F5090 | TechnoClass::PerCellProcess | Per-cell movement processing (reveal, temporal, bridge check) |
| 100 | 0x190  | 0x005F5C20 | ObjectClass::CreateRadialIndicator | Create radial visual indicator |
| 101 | 0x194  | 0x006F4AB0 | TechnoClass::Receive_Radio | Handle radio message from linked object |
| 102 | 0x198  | 0x006F4960 | TechnoClass::DiscoveredBy | Called when unit is discovered by a house |
| 103 | 0x19C  | 0x005F43F0 | ObjectClass::CanPlayerDo_19C | Check what player can do |
| 104 | 0x1A0  | 0x005F4400 | ObjectClass::CanPlayerDo_1A0 | Check what player can do |
| 105 | 0x1A4  | 0x005F6B50 | ObjectClass::IsStealthed | Check if cloaked/stealthed |
| 106 | 0x1A8  | 0x005F4410 | ObjectClass::UpdatePosition | Update position on map |
| 107 | 0x1AC  | 0x004264C0 | ObjectClass::PassMessage_1AC | Receive message stub |
| 108 | 0x1B0  | 0x004264D0 | ObjectClass::PassMessage_1B0 | Click message stub |
| 109 | 0x1B4  | 0x005F6940 | ObjectClass::Set_Raw_Coords | Set raw X/Y/Z coordinates |
| 110 | 0x1B8  | 0x0041BEA0 | ObjectClass::Get_Cell_Packed | Get packed cell coordinates |
| 111 | 0x1BC  | 0x005F6960 | ObjectClass::GetOccupiedCell | Get the cell this object occupies |
| 112 | 0x1C0  | 0x005F69C0 | ObjectClass::GetOccupiedCellClass | Get CellClass for occupied cell |
| 113 | 0x1C4  | 0x005F6A10 | ObjectClass::GetOccupiedCellClass2 | Get CellClass variant |
| 114 | 0x1C8  | 0x005F5F40 | ObjectClass::GetHeight | Get current Z height |
| 115 | 0x1CC  | 0x005F5FA0 | FootClass::Set_Height_On_Bridge | Set height adjusted for bridge |
| 116 | 0x1D0  | 0x005F5F30 | ObjectClass::GetHeight_1D0 | Get height variant |
| 117 | 0x1D4  | 0x0070C5B0 | TechnoClass::IsWarpingOut | Check if being chrono-warped out |
| 118 | 0x1D8  | 0x0070C5C0 | TechnoClass::IsBeingWarped | Check if target of warp effect |
| 119 | 0x1DC  | 0x0070C5D0 | TechnoClass::IsUnderTemporal | Check if temporal weapon is attached (ptr at +0x274) |
| 120 | 0x1E0  | 0x0070C5F0 | TechnoClass::IsNotWarping | Check if NOT currently warping |
| 121 | 0x1E4  | 0x00705D70 | TechnoClass::DrawSHP | Draw SHP sprite for this techno |
| 122 | 0x1E8  | 0x005B35E0 | MissionClass::Queue_Mission | Queue a new mission |
| 123 | 0x1EC  | 0x005B3570 | MissionClass::Commence | Begin executing current mission |
| 124 | 0x1F0  | 0x005B2FD0 | MissionClass::Assign_Mission | Directly assign a mission |
| 125 | 0x1F4  | 0x007013A0 | TechnoClass::Assign_Target | Assign primary attack target |
| 126 | 0x1F8  | 0x007013E0 | TechnoClass::Assign_Destination | Assign movement destination |
| 127 | 0x1FC  | 0x005B3A10 | MissionClass::Is_Mission_Suspended | Check if current mission is suspended |

### Mission Handler Slots (entries 128-156)

These are the per-mission virtual methods. Most return 0x1C2 (default tick delay).
In TechnoClass they are mostly stubs; subclasses (FootClass, InfantryClass, etc.) override them.

| Idx | Offset | Address    | Name | Description |
|-----|--------|------------|------|-------------|
| 128 | 0x200  | 0x004E0140 | Mission_Sleep | Sleep/idle mission handler |
| 129 | 0x204  | 0x005B2E10 | Mission_Default | Default mission handler (returns 0x1C2 = 450 ticks) |
| 130 | 0x208  | 0x005B2E20 | Mission_Attack | Attack mission handler |
| 131 | 0x20C  | 0x005B2E30 | Mission_Move | Move mission handler |
| 132 | 0x210  | 0x005B2E40 | Mission_Retreat | Retreat mission handler |
| 133 | 0x214  | 0x005B2E50 | Mission_Guard | Guard mission handler |
| 134 | 0x218  | 0x005B2E60 | Mission_Sticky | Sticky/hold position handler |
| 135 | 0x21C  | 0x005B2E70 | Mission_Enter | Enter transport handler |
| 136 | 0x220  | 0x005B2E80 | Mission_Capture | Capture building handler |
| 137 | 0x224  | 0x005B2E90 | Mission_Eaten | Eaten/consumed handler |
| 138 | 0x228  | 0x005B2EA0 | Mission_Harvest | Harvest ore handler |
| 139 | 0x22C  | 0x005B2EB0 | Mission_AreaGuard | Area guard/patrol handler |
| 140 | 0x230  | 0x005B2EC0 | Mission_Return | Return to base handler |
| 141 | 0x234  | 0x005B2ED0 | Mission_Stop | Stop mission (returns 0x1C2) |
| 142 | 0x238  | 0x005B2EE0 | Mission_Ambush | Ambush handler |
| 143 | 0x23C  | 0x005B2EF0 | Mission_Hunt | Hunt/seek enemies handler (returns 0x1C2) |
| 144 | 0x240  | 0x005B2F00 | Mission_Unload | Unload passengers handler |
| 145 | 0x244  | 0x005B2F10 | Mission_Sabotage | Sabotage building handler |
| 146 | 0x248  | 0x005B2F20 | Mission_Construction | Building construction handler |
| 147 | 0x24C  | 0x005B2F30 | Mission_Selling | Building selling handler |
| 148 | 0x250  | 0x005B2F40 | Mission_Repair | Repair handler |
| 149 | 0x254  | 0x005B2F50 | Mission_Rescue | Rescue handler |
| 150 | 0x258  | 0x005B2F60 | Mission_Missile | Missile launch handler |
| 151 | 0x25C  | 0x005B2F70 | Mission_Harmless | Harmless/passive handler |
| 152 | 0x260  | 0x005B2F80 | Mission_Open | Open (gate/door) handler |
| 153 | 0x264  | 0x005B2F90 | Mission_Patrol | Patrol handler |
| 154 | 0x268  | 0x005B2FA0 | Mission_ParaDropApproach | Paradrop approach handler |
| 155 | 0x26C  | 0x005B2FB0 | Mission_ParaDropOverfly | Paradrop overfly handler |
| 156 | 0x270  | 0x005B2FC0 | Mission_Wait | Wait handler |

### RadioClass Methods (entries 157-160)

| Idx | Offset | Address    | Name | Description |
|-----|--------|------------|------|-------------|
| 157 | 0x274  | 0x0065ACB0 | RadioClass::Transmit_Radio_ToFirst | Send radio message to first contact |
| 158 | 0x278  | 0x0065AAA0 | RadioClass::Transmit_Radio | Send radio message (general) |
| 159 | 0x27C  | 0x0065A970 | RadioClass::Transmit_Radio_Impl | Radio transmission implementation |
| 160 | 0x280  | 0x0065ACE0 | RadioClass::Broadcast_Radio_ToAll | Broadcast radio message to all contacts |

### TechnoClass-Specific Methods (entries 161-308)

| Idx | Offset | Address    | Name | Description |
|-----|--------|------------|------|-------------|
| 161 | 0x284  | 0x0041BEE0 | ReturnFalse_284 | Stub returning false |
| 162 | 0x288  | 0x0070C5A0 | TechnoClass::HasStealthAbility | Check if has cloak/stealth capability |
| 163 | 0x28C  | 0x006F3280 | TechnoClass::CanBeTargeted | Check if can be targeted (not in sleep/harmless/paralyze mission) |
| 164 | 0x290  | 0x00459D80 | ReturnFalse_290 | Stub |
| 165 | 0x294  | 0x0070BE80 | TechnoClass::IsReadyToCloak | Check if cloaking conditions are met |
| 166 | 0x298  | 0x006F9E10 | TechnoClass::PreAI | Pre-AI tick processing |
| 167 | 0x29C  | 0x0041BEF0 | ReturnFalse_29C | Stub |
| 168 | 0x2A0  | 0x006FBDC0 | TechnoClass::CanAutoCloak | Check if auto-cloaking is allowed |
| 169 | 0x2A4  | 0x006FBC90 | TechnoClass::ShouldUncloak | Determine if should uncloak (target nearby, attacking, etc.) |
| 170 | 0x2A8  | 0x004E0150 | ReturnFalse_2A8 | Stub |
| 171 | 0x2AC  | 0x00701120 | TechnoClass::CanHaveKickout | Check if unit can kick out of building |
| 172 | 0x2B0  | 0x0070C620 | TechnoClass::IsNotAtDestination | Check if still moving toward destination |
| 173 | 0x2B4  | 0x00708BC0 | TechnoClass::GetTargetingData_2B4 | Targeting data accessor |
| 174 | 0x2B8  | 0x00708C30 | TechnoClass::GetTargetingData_2B8 | Targeting data accessor |
| 175 | 0x2BC  | 0x0070ADA0 | TechnoClass::UpdateSensors_2BC | Update sensor coverage |
| 176 | 0x2C0  | 0x00708B40 | TechnoClass::GetTimerData_2C0 | Timer/rate data |
| 177 | 0x2C4  | 0x00459D90 | ReturnFalse_2C4 | Stub |
| 178 | 0x2C8  | 0x006FDA00 | TechnoClass::GetTurretFacing | Get turret facing direction (with rotation calculation) |
| 179 | 0x2CC  | 0x00707F60 | TechnoClass::GetGuardRange | Get guard scanning range |
| 180 | 0x2D0  | 0x006F3950 | TechnoClass::GetWeaponRange_2D0 | Get weapon range (another variant) |
| 181 | 0x2D4  | 0x0041BF00 | ReturnFalse_2D4 | Stub |
| 182 | 0x2D8  | 0x0041BF10 | ReturnFalse_2D8 | Stub |
| 183 | 0x2DC  | 0x0041BF20 | ReturnFalse_2DC | Stub |
| 184 | 0x2E0  | 0x0070D980 | TechnoClass::ShouldUncloak_2E0 | Another uncloak check variant |
| 185 | 0x2E4  | 0x006F3330 | TechnoClass::SelectWeaponAgainst | Choose best weapon vs target (primary/secondary) |
| 186 | 0x2E8  | 0x006F3820 | TechnoClass::CanTargetVsTurretLock | Check weapon eligibility based on LandTargeting/NavalTargeting |
| 187 | 0x2EC  | 0x00704350 | TechnoClass::GetZAdjust | Calculate Z-coordinate draw adjustment (bridge, slope) |
| 188 | 0x2F0  | 0x00459DA0 | ReturnFalse_2F0 | Stub |
| 189 | 0x2F4  | 0x00459DB0 | ReturnFalse_2F4 | Stub |
| 190 | 0x2F8  | 0x00459DC0 | ReturnFalse_2F8 | Stub |
| 191 | 0x2FC  | 0x0070AD50 | TechnoClass::AddSensors_2FC | Add sensor data |
| 192 | 0x300  | 0x006F3D60 | TechnoClass::GetTurretLocation | Get 3D position of turret (matrix transform for VXL) |
| 193 | 0x304  | 0x00708C10 | TechnoClass::GetBodyFacing | Get body facing direction |
| 194 | 0x308  | 0x00708D70 | TechnoClass::GetTurretFacing_Raw | Get raw turret facing value |
| 195 | 0x30C  | 0x00707D20 | TechnoClass::GetVoiceResponse | Get voice response (guard, move, attack) based on vet level |
| 196 | 0x310  | 0x00700D10 | TechnoClass::CanEnterTransport | Check if can enter a transport |
| 197 | 0x314  | 0x00700D50 | TechnoClass::CanEnterCell | Check if can move to a cell |
| 198 | 0x318  | 0x006FCFA0 | TechnoClass::GetROF | Calculate rate of fire (base + veterancy + crate + garrisoned) |
| 199 | 0x31C  | 0x00707E60 | TechnoClass::GetRearmDelay | Get rearm delay between shots |
| 200 | 0x320  | 0x00459DD0 | ReturnFalse_320 | Stub |
| 201 | 0x324  | 0x0070D1D0 | TechnoClass::SetDrawHealthBarsFlags | Set/clear health bar draw flag |
| 202 | 0x328  | 0x0070D420 | TechnoClass::StopAllTargeting_328 | Clear all targeting data |
| 203 | 0x32C  | 0x0070D460 | TechnoClass::StopAllTargeting_32C | Clear all targeting data variant |
| 204 | 0x330  | 0x0041BF30 | ReturnFalse_330 | Stub |
| 205 | 0x334  | 0x00459DE0 | ReturnFalse_334 | Stub |
| 206 | 0x338  | 0x0070F8F0 | TechnoClass::OnCapture | Handle capture/mind-control event |
| 207 | 0x33C  | 0x00459DF0 | ReturnFalse_33C | Stub |
| 208 | 0x340  | 0x00459E00 | ReturnFalse_340 | Stub |
| 209 | 0x344  | 0x00459E10 | ReturnFalse_344 | Stub |
| 210 | 0x348  | 0x00459E20 | ReturnFalse_348 | Stub |
| 211 | 0x34C  | 0x00459E30 | ReturnFalse_34C | Stub |
| 212 | 0x350  | 0x00701190 | TechnoClass::SetTarget_350 | Set targeting data |
| 213 | 0x354  | 0x00708D90 | TechnoClass::GetWeaponStruct_354 | Get weapon struct for primary weapon |
| 214 | 0x358  | 0x00709020 | TechnoClass::GetPrimaryWeapon | Get primary weapon pointer (with vet/elite upgrade) |
| 215 | 0x35C  | 0x00709060 | TechnoClass::GetSecondaryWeapon | Get secondary weapon pointer (with vet/elite upgrade) |
| 216 | 0x360  | 0x00708EB0 | TechnoClass::GetWeapon_Veteran | Get veteran upgrade weapon |
| 217 | 0x364  | 0x00708DC0 | TechnoClass::GetWeaponStruct_364 | Get weapon struct for secondary |
| 218 | 0x368  | 0x00708FC0 | TechnoClass::GetElitePrimaryWeapon | Get elite upgrade primary weapon |
| 219 | 0x36C  | 0x00708E00 | TechnoClass::GetWeaponStruct_36C | Get weapon struct variant |
| 220 | 0x370  | 0x007090A0 | TechnoClass::GetEliteSecondaryWeapon | Get elite upgrade secondary weapon |
| 221 | 0x374  | 0x006FFE00 | TechnoClass::What_Action_Evaluate | Evaluate action for cell (detailed) |
| 222 | 0x378  | 0x006FFBE0 | TechnoClass::CanAfford | Check if owning house can afford action |
| 223 | 0x37C  | 0x0070EFD0 | TechnoClass::IsUnderEMP | Check if disabled by EMP |
| 224 | 0x380  | 0x00459E40 | ReturnFalse_380 | Stub (IsInGarrison?) |
| 225 | 0x384  | 0x0041BF80 | ReturnFalse_384 | Stub |
| 226 | 0x388  | 0x0041BF90 | ReturnFalse_388 | Stub |
| 227 | 0x38C  | 0x0070EFE0 | TechnoClass::GetEMPData | Get EMP damage/state info |
| 228 | 0x390  | 0x0070D670 | TechnoClass::GetIdleAnim | Get idle animation pointer |
| 229 | 0x394  | 0x00710670 | TechnoClass::FreeMindControlledChain | Release chain of mind-controlled units |
| 230 | 0x398  | 0x0070EF00 | TechnoClass::CanEMPAffect | Check if EMP can affect this |
| 231 | 0x39C  | 0x00709820 | TechnoClass::Retaliate_And_Scan | Auto-retaliate against attacker and scan for threats |
| 232 | 0x3A0  | 0x006FCD40 | TechnoClass::StopFiring | Stop all weapons fire, kill spawns, clear targets |
| 233 | 0x3A4  | 0x006F7660 | TechnoClass::IsInWeaponRange | Check if target is within primary weapon range |
| 234 | 0x3A8  | 0x006F77B0 | TechnoClass::CanFireAt | Can this fire at given target (full check) |
| 235 | 0x3AC  | 0x006F7780 | TechnoClass::CanFireAtTarget | Can fire at target (simplified) |
| 236 | 0x3B0  | 0x006F7930 | TechnoClass::PreFire | Pre-fire checks/setup |
| 237 | 0x3B4  | 0x006F78D0 | TechnoClass::FireWeapon | Dispatch fire to AssignTarget or AssignDestination |
| 238 | 0x3B8  | 0x004C9150 | Stub::ReturnZero | Stub (PostFire?) |
| 239 | 0x3BC  | 0x006FC090 | TechnoClass::GetFireError_3BC | Detailed fire error code |
| 240 | 0x3C0  | 0x006FC0B0 | TechnoClass::GetFireError | Get why firing would fail (out of range, no ammo, etc.) |
| 241 | 0x3C4  | 0x006F8DF0 | TechnoClass::Greatest_Threat | Find highest-priority threat in range |
| 242 | 0x3C8  | 0x006FCDB0 | TechnoClass::Set_ArchiveTarget | Set archive/remembered target |
| 243 | 0x3CC  | 0x006FDD50 | TechnoClass::Fire_At | Actually fire weapon at target (create projectile) |
| 244 | 0x3D0  | 0x0070F850 | TechnoClass::StopAndGuard | Stop firing, clear target, go to guard mission |
| 245 | 0x3D4  | 0x007014A0 | TechnoClass::ChangeOwner | Transfer ownership to another house |
| 246 | 0x3D8  | 0x0070B280 | TechnoClass::UpdateRocking | Update ship/aircraft rocking motion |
| 247 | 0x3DC  | 0x00459E50 | ReturnFalse_3DC | Stub |
| 248 | 0x3E0  | 0x0070DD50 | TechnoClass::HasActiveWeapon | Check if has any usable weapon |
| 249 | 0x3E4  | 0x0070DD70 | TechnoClass::GetBurstIndex | Get current burst fire index |
| 250 | 0x3E8  | 0x0070DD90 | TechnoClass::GetBurstDelay | Get delay between burst shots |
| 251 | 0x3EC  | 0x0070DDA0 | TechnoClass::GetBurstCount | Get number of shots per burst |
| 252 | 0x3F0  | 0x0070E120 | TechnoClass::GetWeaponForTarget | Get appropriate weapon struct for a target |
| 253 | 0x3F4  | 0x0070E1A0 | TechnoClass::GetWeaponForIndex | Get weapon struct by index (0=primary, 1=secondary) |
| 254 | 0x3F8  | 0x0070E140 | TechnoClass::GetWeaponTypeForTarget | Get WeaponTypeClass for target |
| 255 | 0x3FC  | 0x0041BFA0 | ReturnFalse_3FC | Stub |
| 256 | 0x400  | 0x0041BFB0 | TechnoClass::HasPassengers | Check if has passengers |
| 257 | 0x404  | 0x0041BFC0 | TechnoClass::GetPassengerCount | Get number of passengers |
| 258 | 0x408  | 0x0041BFD0 | TechnoClass::GetPassengerHP | Get total passenger HP |
| 259 | 0x40C  | 0x00701410 | TechnoClass::EngineerRepair | Handle engineer repair action |
| 260 | 0x410  | 0x006FB740 | TechnoClass::CloakingTick | Per-tick cloaking state machine |
| 261 | 0x414  | 0x006FB170 | TechnoClass::UpdateCloakShroud | Update cloak detection/shroud area |
| 262 | 0x418  | 0x006FB470 | TechnoClass::RemoveCloakShroud | Remove cloak detection area |
| 263 | 0x41C  | 0x0070B570 | TechnoClass::RockingUpdate | Update rocking amplitude and phase |
| 264 | 0x420  | 0x006F4EB0 | TechnoClass::DoUncloak | Force uncloak transition |
| 265 | 0x424  | 0x006FB010 | TechnoClass::CloakDetect | Check for cloak-detecting units nearby |
| 266 | 0x428  | 0x0041BFE0 | ReturnFalse_428 | Stub |
| 267 | 0x42C  | 0x00705CA0 | TechnoClass::DrawSHP_42C | Draw SHP frame (variant) |
| 268 | 0x430  | 0x00705D50 | TechnoClass::DrawVoxel | Draw VXL voxel model |
| 269 | 0x434  | 0x0041BFF0 | ReturnFalse_434 | Stub |
| 270 | 0x438  | 0x00459E60 | ReturnFalse_438 | Stub |
| 271 | 0x43C  | 0x0070ED80 | TechnoClass::ModifyCloakDrawFlags | Modify draw flags based on cloak state (alpha, etc.) |
| 272 | 0x440  | 0x0070EE30 | TechnoClass::ProcessCloakDraw | Process cloaking visual effects for rendering |
| 273 | 0x444  | 0x00706640 | TechnoClass::Draw | Main draw function (dispatch to SHP/VXL/etc.) |
| 274 | 0x448  | 0x006F60C0 | TechnoClass::DrawOverlay | Draw overlay (empty in TechnoClass) |
| 275 | 0x44C  | 0x006F64A0 | TechnoClass::DrawHealthBar | Draw health bar above unit |
| 276 | 0x450  | 0x00709A90 | TechnoClass::DrawPipScalePips_450 | Draw pip indicators (ammo, tiberium, passengers) |
| 277 | 0x454  | 0x0070A990 | TechnoClass::DrawVeterancyPips_454 | Draw veterancy pip indicators |
| 278 | 0x458  | 0x0070AA60 | TechnoClass::DrawExtraInfo | Draw extra UI info (group numbers, etc.) |
| 279 | 0x45C  | 0x007036C0 | TechnoClass::StartUncloaking | Begin uncloak sequence |
| 280 | 0x460  | 0x00703770 | TechnoClass::StartCloaking | Begin cloak sequence |
| 281 | 0x464  | 0x0070D190 | TechnoClass::ClampSpeed | Clamp speed to max (checks LOCO flag at +0xF0 bit 1) |
| 282 | 0x468  | 0x0041C000 | ReturnFalse_468 | Stub |
| 283 | 0x46C  | 0x0070E280 | TechnoClass::GetIronCurtainState | Get Iron Curtain remaining time/state |
| 284 | 0x470  | 0x0041C030 | ReturnFalse_470 | Stub |
| 285 | 0x474  | 0x007099E0 | TechnoClass::DrawPipScale | Draw pip scale helper |
| 286 | 0x478  | 0x0041C040 | ReturnFalse_478 | Stub |
| 287 | 0x47C  | 0x00709A20 | TechnoClass::GetPipCount_47C | Get number of pips to draw |
| 288 | 0x480  | 0x00709A30 | TechnoClass::SetTarget_480 | Set target (with weapon index) |
| 289 | 0x484  | 0x00709A40 | TechnoClass::PostArrivalMissionDispatch | Post-arrival mission dispatch (base fallback): detach temporal link, check EMP, convoy-dequeue, queue next mission — corrected 2026-05-28: was "GetTarget_484 / Get current target"; header note added 2026-05-19 already flagged WRONG; table body now updated — ROOT_CAUSE: INFERENCE_HARDENED; see TECHNO_VTABLE_0x484_DRIVE_PROCESS_ARRIVAL_GHIDRA_REPORT.md |
| 290 | 0x488  | 0x0070AF50 | TechnoClass::UpdateReveal | Reveal map around unit (sight range + vet bonuses) |
| 291 | 0x48C  | 0x0070B1D0 | TechnoClass::ReReveal | Re-reveal surrounding cells (after movement) |
| 292 | 0x490  | 0x004C9150 | Stub::ReturnZero | Stub |
| 293 | 0x494  | 0x0070CC90 | TechnoClass::RegisterOnRadar | Register blip on radar display |
| 294 | 0x498  | 0x0070CCC0 | TechnoClass::UnregisterFromRadar | Remove blip from radar |
| 295 | 0x49C  | 0x0070CCF0 | TechnoClass::IdleActionTick | Per-tick idle action processing |
| 296 | 0x4A0  | 0x0070D990 | TechnoClass::IdleAnimDispatch | Dispatch idle animation |
| 297 | 0x4A4  | 0x0070F000 | TechnoClass::EMP_Handler_4A4 | EMP effect handler |
| 298 | 0x4A8  | 0x0070F010 | TechnoClass::EMP_Handler_4A8 | EMP effect handler |
| 299 | 0x4AC  | 0x0070F020 | TechnoClass::EMP_Handler_4AC | EMP effect handler |
| 300 | 0x4B0  | 0x0070F030 | TechnoClass::EMP_Handler_4B0 | EMP effect handler |
| 301 | 0x4B4  | 0x0070F040 | TechnoClass::EMP_Handler_4B4 | EMP effect handler |
| 302 | 0x4B8  | 0x0070F050 | TechnoClass::EMP_Handler_4B8 | EMP effect handler |
| 303 | 0x4BC  | 0x0070F070 | TechnoClass::EMP_Handler_4BC | EMP effect handler |
| 304 | 0x4C0  | 0x0070F090 | TechnoClass::EMP_Handler_4C0 | EMP effect handler |
| 305 | 0x4C4  | 0x0070F0E0 | TechnoClass::EMP_Handler_4C4 | EMP effect handler |
| 306 | 0x4C8  | 0x0070F0F0 | TechnoClass::EMP_Handler_4C8 | EMP effect handler |
| 307 | 0x4CC  | 0x0070F100 | TechnoClass::EMP_Handler_4CC | EMP effect handler |
| 308 | 0x4D0  | 0x0070F110 | TechnoClass::EMP_Handler_4D0 | EMP effect handler (last entry) |

---

## Key Method Groups Summary

### Combat / Weapons
- **SelectWeaponAgainst** (idx 185, 0x2E4): Choose primary vs secondary weapon for a target
- **CanFireAt** (idx 234, 0x3A8): Full fire eligibility check
- **CanFireAtTarget** (idx 235, 0x3AC): Simplified fire check
- **GetFireError** (idx 240, 0x3C0): Detailed reason why firing would fail
- **Fire_At** (idx 243, 0x3CC): Actually launch projectile
- **GetROF** (idx 198, 0x318): Rate of fire calculation with veterancy/crate bonuses
- **Greatest_Threat** (idx 241, 0x3C4): AI threat evaluation/target selection
- **Retaliate_And_Scan** (idx 231, 0x39C): Auto-retaliate + passive scan
- **InRange** (idx 89, 0x164): Range check to target
- **GetWeaponRange** (idx 90, 0x168): Effective weapon range

### Weapon Accessors (idx 213-220)
- Primary/Secondary weapon structs with vet/elite upgrade selection
- GetPrimaryWeapon (idx 214), GetSecondaryWeapon (idx 215)
- GetElitePrimaryWeapon (idx 218), GetEliteSecondaryWeapon (idx 220)

### Cloaking / Stealth
- **HasStealthAbility** (idx 162, 0x288): Has cloak capability
- **CanAutoCloak** (idx 168, 0x2A0): Can auto-cloak
- **ShouldUncloak** (idx 169, 0x2A4): Should break cloak
- **CloakingTick** (idx 260, 0x410): Per-tick cloak state machine
- **StartCloaking** (idx 280, 0x460): Begin cloak
- **StartUncloaking** (idx 279, 0x45C): Begin uncloak
- **DoUncloak** (idx 264, 0x420): Force uncloak
- **ModifyCloakDrawFlags** (idx 271, 0x43C): Visual alpha for cloaked state

### Iron Curtain / Special States
- **IronCurtain** (idx 85, 0x154): Apply invulnerability
- **IsIronCurtainActive** (idx 88, 0x160): Check invuln active
- **IsWarpingOut** (idx 117, 0x1D4): Chrono warp state
- **IsBeingWarped** (idx 118, 0x1D8): Being warped state
- **IsUnderEMP** (idx 223, 0x37C): EMP disabled state
- **IsUnderTemporal** (idx 119, 0x1DC): Temporal weapon attached

### Targeting / AI
- **Assign_Target** (idx 125, 0x1F4): Set attack target
- **Assign_Destination** (idx 126, 0x1F8): Set move destination
- **Set_ArchiveTarget** (idx 242, 0x3C8): Remember target for return

### Vision / Sensors
- **UpdateReveal** (idx 290, 0x488): Map reveal with sight range
- **ReReveal** (idx 291, 0x48C): Re-reveal after movement
- **RegisterOnRadar** (idx 293, 0x494): Radar blip management
- **UpdateSensors** (idx 72, 0x120): Sensor map updates

### Drawing / Rendering
- **Draw** (idx 273, 0x444): Main draw dispatch
- **DrawBehind** (idx 67, 0x10C): Background elements
- **DrawExtras** (idx 68, 0x110): UI overlays
- **DrawHealthBar** (idx 275, 0x44C): Health bar
- **DrawPipScalePips** (idx 276, 0x450): Pip indicators
- **DrawVeterancyPips** (idx 277, 0x454): Vet pips
- **DrawSHP** (idx 121, 0x1E4): SHP sprite rendering
- **DrawVoxel** (idx 268, 0x430): VXL model rendering
- **GetZAdjust** (idx 187, 0x2EC): Z-offset for draw sorting

### Movement / State
- **CanMove** (idx 40, 0x0A0): Can currently move
- **PerCellProcess** (idx 99, 0x18C): Per-cell movement events
- **GetTurretFacing** (idx 178, 0x2C8): Turret direction
- **GetBodyFacing** (idx 193, 0x304): Body direction
- **ChangeOwner** (idx 245, 0x3D4): Ownership transfer

### Lifecycle
- **Limbo** (idx 53, 0x0D4): Remove from map
- **Unlimbo** (idx 54, 0x0D8): Place on map
- **Destroy** (idx 55, 0x0DC): Destruction sequence
- **RecordKill** (idx 56, 0x0E0): Kill credit/veterancy
- **ReceiveDamage** (idx 91, 0x16C): Damage processing

### Secondary Vtables

The constructor also sets three secondary vtables:
- `this+4` = `0x007F4944` (7 entries before primary vtable)
- `this+8` = `0x007F493C` 
- `this+12` = `0x007F4934`

These implement secondary COM interfaces (IRTTITypeInfo, INoticeSink, INoticeSource).
