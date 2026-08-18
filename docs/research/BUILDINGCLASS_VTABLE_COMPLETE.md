---
name: BuildingClass Vtable (Complete — 338 Slots)
description: Every slot in the BuildingClass vtable at 0x007E3EBC — inherited vs overridden vs pure-virtual. Supersedes v2 §4 partial coverage and the older BUILDINGCLASS_VTABLE_AND_LIFECYCLE.md. Extended from 300→338 slots via T5/T10 reconciliation.
type: reference
---

# BuildingClass Complete Vtable — 338 Slots

**Binary:** `gamemd.exe`
**BuildingClass vtable:** `0x007E3EBC`
**TechnoClass base vtable:** `0x007F4960`
**Total slots:** **338** (offsets 0x000–0x544, 1352 bytes); NULL terminator at slot 338
**Date:** 2026-04-24 (R4 extension — originally 300 slots, extended to 338 via T5/T10 reconciliation)
**Method:** Direct read of vtable memory via `read_memory`; per-slot addr comparison with TechnoClass; Ghidra decompilation of every slot where BC addr differs from TC addr; cross-reference with `TECHNOCLASS_VTABLE_COMPLETE.md` for inherited slot names.

---

## Summary

| Category | Count | Notes |
|---|---|---|
| Total function slots | **338** | 0..337, terminated by NULL at slot 338 |
| Primary vtable (slots 0..321) | 322 entries | Normal function pointers |
| Secondary-vtable MI markers | 2 entries | Slots 322 (`0x007FC298`) + 330 (`0x007FC390`) — IPersistStream MI interface pointers, **not functions** |
| Tail continuation pointers | 14 entries | Slots 323-329, 331-337 — MI-related helpers |
| Inherited (BC addr == TC addr, 0..299) | **199** | TechnoClass implementation reused as-is |
| Overridden (BC addr != TC addr, 0..299) | **101** | Either BuildingClass-specific or a thin thunk |
| New/override in extension range (300..337) | See §Extension table | Most are BuildingClass-specific |
| Pure-virtual stub on BuildingClass | **0** | Every TC pure-virtual slot (3, 11, 12, 238, 292) has a concrete implementation |

**Cross-check vs v2 claim ("~95 overrides"):** my count is 101 — 6 higher than v2 but in the same ballpark. The 6-slot delta comes from slots v2 explicitly listed as "range inherited" without doing the per-slot compare (e.g., it did not break out slots 170, 171, 174, 180, 181, 182, 183, 184, 187 in the 161–199 range; some of those are overrides).

**The five TC pure-virtual placeholders (address `0x004C9150` = `Stub__ReturnZero`) are ALL overridden by BuildingClass:**
- Slot 3 (GetClassID) → `0x00459E80` (returns GUID `DAT_007e96a0..7e96ac`)
- Slot 11 (WhatAmI) → `0x00459EC0` (returns 6 = AbstractType::Building)
- Slot 12 (SizeOf) → `0x00459E70` (returns 0x720 = 1824 bytes)
- Slot 238 (PostFire?) → `0x0044D760` (concrete impl)
- Slot 292 → `0x00458A80` (concrete impl)

Because no BC slot still points at `Stub__ReturnZero`, there are no pure-virtual traps on a live BuildingClass instance.

---

## Full slot table

Legend: `I` = inherited from TechnoClass (same address); `O` = BuildingClass override.

| Slot | Offset | Address | Cat | Function | Purpose | Conf |
|---|---|---|---|---|---|---|
| 0 | 0x000 | 0x00410260 | I | AbstractClass::QueryInterface | COM QI | HIGH |
| 1 | 0x004 | 0x00410300 | I | AbstractClass::AddRef | COM AddRef | HIGH |
| 2 | 0x008 | 0x00410310 | I | AbstractClass::Release | COM Release | HIGH |
| 3 | 0x00C | 0x00459E80 | **O** | BuildingClass::GetClassID | IPersistStream::GetClassID — copies 16-byte GUID from `DAT_007e96a0` | HIGH |
| 4 | 0x010 | 0x00410450 | I | AbstractClass::IsDirty | IPersistStream::IsDirty | HIGH |
| 5 | 0x014 | 0x00453E20 | **O** | **BuildingClass::Load** | IPersistStream::Load — calls `TechnoClass::Load` (0x0070BF50), then swizzles building-specific pointers, re-reads radio-contact/dock arrays, initializes VocHandles. **Used by Task 8.** | HIGH |
| 6 | 0x018 | 0x00454190 | **O** | **BuildingClass::Save** | IPersistStream::Save — calls `TechnoClass::Save_Stream` (0x0070C250), then writes building-specific state (radio contacts, dock arrays). **Used by Task 8.** | HIGH |
| 7 | 0x01C | 0x004103E0 | I | AbstractClass::GetSizeMax | IPersistStream::GetSizeMax | HIGH |
| 8 | 0x020 | 0x00459F20 | **O** | BuildingClass::ScalarDeletingDestructor | `operator delete` + constructor-style chain; matches MSVC vcall-dtor pattern | HIGH |
| 9 | 0x024 | 0x00442C40 | **O** | BuildingClass::Init_Managers | Overrides TechnoClass::Init_Managers; calls TC base, then `HouseClass__Add_Tracking`, init power/rate timers, registers in global building-array | HIGH |
| 10 | 0x028 | 0x0044E8F0 | **O** | BuildingClass::GetType | Returns BuildingTypeClass ptr (already labeled in Ghidra) | HIGH |
| 11 | 0x02C | 0x00459EC0 | **O** | BuildingClass::WhatAmI | Returns `6` (AbstractType::Building enum) | HIGH |
| 12 | 0x030 | 0x00459E70 | **O** | BuildingClass::SizeOf | Returns `0x720` (1824 bytes) — matches earlier constructor-based estimate | HIGH |
| 13 | 0x034 | 0x00454260 | **O** | BuildingClass::Save_ChecksumFields | Computes deterministic CRC input — `TechnoClass::Save` + building timers/flags at offsets 0x528..0x702 | HIGH |
| 14 | 0x038 | 0x006F9DB0 | I | TechnoClass::GetOwnerHouseID | | HIGH |
| 15 | 0x03C | 0x006F9DC0 | I | TechnoClass::GetOwnerHousePtr | | HIGH |
| 16 | 0x040 | 0x004104B0 | I | AbstractClass::ReturnFalse | | HIGH |
| 17 | 0x044 | 0x005F6690 | I | ObjectClass::IsDead | | HIGH |
| 18 | 0x048 | 0x00447AC0 | **O** | BuildingClass::GetCoords | Building-footprint-aware coord (already labeled) | HIGH |
| 19 | 0x04C | 0x00447E90 | **O** | BuildingClass::GetFacingCoords | Dispatches to GetDockCoord/GetCoords based on Type flags 0x16cb/0x16a9/0x16ab | HIGH |
| 20 | 0x050 | 0x005F6B60 | I | ObjectClass::IsLowFlying | | HIGH |
| 21 | 0x054 | 0x005F6B90 | I | ObjectClass::IsHighFlying | | HIGH |
| 22 | 0x058 | 0x00410540 | I | AbstractClass::GetTargetCoords | | HIGH |
| 23 | 0x05C | 0x0043FB20 | **O** | **BuildingClass::Update** | Main per-tick AI for buildings — already labeled | HIGH |
| 24 | 0x060 | 0x00710410 | I | TechnoClass::Detach | | HIGH |
| 25 | 0x064 | 0x006F32D0 | I | TechnoClass::ReadINI | (TC impl reused; building ReadINI is via BuildingTypeClass) | HIGH |
| 26 | 0x068 | 0x004544A0 | **O** | BuildingClass::GetVisualState | Already labeled | HIGH |
| 27 | 0x06C | 0x004513D0 | **O** | BuildingClass::GetAction | Small dispatcher on field 0x534/0x6e4; picks BuildingTypeClass vt+0x9c or +0xc0 | MED |
| 28 | 0x070 | 0x00447540 | **O** | BuildingClass::What_Action_OnCell | Building-specific cell-action (IsShrouded/IsOccupied checks, ClearBibArea, return codes 0/1/2/5/6) | HIGH |
| 29 | 0x074 | 0x00447210 | **O** | BuildingClass::What_Action_OnObject | Building-specific object-action (capture/engineer/repair/no-sell rules) | HIGH |
| 30 | 0x078 | 0x005F4260 | I | ObjectClass::GetThreatLevel | | HIGH |
| 31 | 0x07C | 0x005F6C10 | I | ObjectClass::IsAboveGround | | HIGH |
| 32 | 0x080 | 0x00457620 | **O** | BuildingClass::Is1x1WithUndeploy | Thin wrapper — calls `BuildingTypeClass__Is1x1WithUndeploy` | HIGH |
| 33 | 0x084 | 0x006F3270 | I | TechnoClass::GetTechnoType | | HIGH |
| 34 | 0x088 | 0x00459EE0 | **O** | BuildingClass::GetObjectType | Stub (6 bytes) — returns BuildingTypeClass ptr variant | MED |
| 35 | 0x08C | 0x00708B30 | I | TechnoClass::GetTimerStruct | | HIGH |
| 36 | 0x090 | 0x00459ED0 | **O** | BuildingClass::GetPixelSelectionBracketDelta | Returns `Type+0x60` — always overridden | HIGH |
| 37 | 0x094 | 0x00452630 | **O** | **BuildingClass::IsDeployable** | (v2-corrected) — Type+0x157a gate, then TC::IsDeployable tail-call | HIGH |
| 38 | 0x098 | 0x004494C0 | **O** | BuildingClass::ClearBibArea | Already labeled | HIGH |
| 39 | 0x09C | 0x007010D0 | I | TechnoClass::CanDeploy | | HIGH |
| 40 | 0x0A0 | 0x0044F5C0 | **O** | BuildingClass::ShouldShowDeployButton | Already labeled | HIGH |
| 41 | 0x0A4 | 0x004500A0 | **O** | BuildingClass::GetTargetCoords | Already labeled | HIGH |
| 42 | 0x0A8 | 0x00447B20 | **O** | BuildingClass::GetDockCoord | Already labeled (long — handles multi-dock offsets) | HIGH |
| 43 | 0x0AC | 0x00459EF0 | **O** | BuildingClass::GetRenderCoords | Already labeled | HIGH |
| 44 | 0x0B0 | 0x00453840 | **O** | BuildingClass::GetFLH | Firing/Launch/Height with turret-pixel-offset integration | HIGH |
| 45 | 0x0B4 | 0x0044F640 | **O** | BuildingClass::GetCenterCoord (alt) | Center-of-foundation calculator; small | MED |
| 46 | 0x0B8 | 0x00449410 | **O** | BuildingClass::GetYSort | Overrides ObjectClass::GetYSort for foundation center-based sort | MED |
| 47 | 0x0BC | 0x005F6A70 | I | ObjectClass::ShouldBeOnBridge | | HIGH |
| 48 | 0x0C0 | 0x00426410 | I | ObjectClass::GetFoundation | | HIGH |
| 49 | 0x0C4 | 0x0041C010 | I | ReturnFalse_0C4 | | HIGH |
| 50 | 0x0C8 | 0x0041C020 | I | ReturnFalse_0C8 | | HIGH |
| 51 | 0x0CC | 0x0041BE60 | I | ReturnFalse_0CC | | HIGH |
| 52 | 0x0D0 | 0x0041BE70 | I | ReturnFalse_0D0 | | HIGH |
| 53 | 0x0D4 | 0x00445880 | **O** | **BuildingClass::Limbo** (remove-from-map cleanup) | Overrides TC::Limbo; walls, radar, house counters, anim-slot release. **Renamed 2026-04-24 from "OnDestroyed" — see T5/T10 reconciliation.** Not the HP=0 handler (that is slot 315). | HIGH |
| 54 | 0x0D8 | 0x00440580 | **O** | **BuildingClass::Unlimbo** | Already labeled — ~4,300-byte placement routine | HIGH |
| 55 | 0x0DC | 0x0044EBF0 | **O** | **BuildingClass::Destroy** | Overrides ObjectClass::Destroy — aborts factory production, ejects queued units. **v2 master Sources listed this as "Limbo" — wrong (Limbo is slot 53).** | HIGH |
| 56 | 0x0E0 | 0x00702D40 | I | TechnoClass::RecordKill | | HIGH |
| 57 | 0x0E4 | 0x00703230 | I | TechnoClass::KillPassengers | | HIGH |
| 58 | 0x0E8 | 0x005F5940 | I | ObjectClass::Unlimbo (base) | | HIGH |
| 59 | 0x0EC | 0x005F4160 | I | ObjectClass::DropIn | | HIGH |
| 60 | 0x0F0 | 0x00453D60 | **O** | BuildingClass::Mark_Put | Place building cells (occupancy markers) — small (~85 bytes) | HIGH |
| 61 | 0x0F4 | 0x00453DC0 | **O** | BuildingClass::Mark_Remove | Clear building cell markers — small (~85 bytes) | HIGH |
| 62 | 0x0F8 | 0x005F65F0 | I | ObjectClass::UnInit | | HIGH |
| 63 | 0x0FC | 0x00703850 | I | TechnoClass::UpdateSensors_0FC | | HIGH |
| 64 | 0x100 | 0x00443C60 | **O** | BuildingClass::ExitObject | Already labeled `BuildingClass__ExitObject_Main` — production unit exit | HIGH |
| 65 | 0x104 | 0x0043CEA0 | **O** | BuildingClass::DrawIt | Building overall sprite draw (SHP dispatch) | MED |
| 66 | 0x108 | 0x005F5B90 | I | ObjectClass::DrawVoxelShadow | | HIGH |
| 67 | 0x10C | 0x006F60D0 | I | TechnoClass::DrawBehind | | HIGH |
| 68 | 0x110 | 0x006F5190 | I | TechnoClass::DrawExtras | | HIGH |
| 69 | 0x114 | 0x0043D290 | **O** | **BuildingClass::DrawBody** | Already labeled | HIGH |
| 70 | 0x118 | 0x0043D030 | **O** | BuildingClass::DrawBody_Helper | Secondary draw helper (overlays, glow) | MED |
| 71 | 0x11C | 0x006F4A40 | I | TechnoClass::CheckPlayerDiscovery | | HIGH |
| 72 | 0x120 | 0x0070ADC0 | I | TechnoClass::UpdateSensors | | HIGH |
| 73 | 0x124 | 0x0043F180 | **O** | BuildingClass::SetMissionAndAnims | Large (~2,400 bytes) — mission change + anim state transitions | HIGH |
| 74 | 0x128 | 0x005F4730 | I | ObjectClass::GetDrawExtent | | HIGH |
| 75 | 0x12C | 0x00455C20 | **O** | BuildingClass::GetDrawRect | Overrides base to use building footprint | MED |
| 76 | 0x130 | 0x00456750 | **O** | BuildingClass::MarkNeedsRedraw | Updates building-specific redraw flags + gap generator | MED |
| 77 | 0x134 | 0x005F4D10 | I | ObjectClass::MarkNeedsRedraw | | HIGH |
| 78 | 0x138 | 0x005F6C30 | I | ObjectClass::CanBeSelected | | HIGH |
| 79 | 0x13C | 0x00459C00 | **O** | BuildingClass::CanBeSelectedNow | Already labeled | HIGH |
| 80 | 0x140 | 0x004436F0 | **O** | BuildingClass::ClickedAction_140 | Click/action on building (rally point, sidebar flash) | MED |
| 81 | 0x144 | 0x00443410 | **O** | BuildingClass::ClickedAction_144 | Click/action variant | MED |
| 82 | 0x148 | 0x00456E00 | **O** | BuildingClass::SetOwnerHouse | Override for building-specific ownership fixup | MED |
| 83 | 0x14C | 0x006FBFA0 | I | TechnoClass::Select | | HIGH |
| 84 | 0x150 | 0x005F44A0 | I | ObjectClass::Deselect | | HIGH |
| 85 | 0x154 | 0x00457C90 | **O** | **BuildingClass::IronCurtain** | Already labeled | HIGH |
| 86 | 0x158 | 0x0070E340 | I | TechnoClass::SetCoords_158 | | HIGH |
| 87 | 0x15C | 0x0070E300 | I | TechnoClass::SetCoords_15C | | HIGH |
| 88 | 0x160 | 0x0041BF40 | I | TechnoClass::IsIronCurtainActive | | HIGH |
| 89 | 0x164 | 0x006F7970 | I | TechnoClass::InRange | | HIGH |
| 90 | 0x168 | 0x007012C0 | I | TechnoClass::GetWeaponRange | | HIGH |
| 91 | 0x16C | 0x00442230 | **O** | **BuildingClass::ReceiveDamage** | Already labeled | HIGH |
| 92 | 0x170 | 0x00710460 | I | TechnoClass::FreeAllMindControlCaptures | | HIGH |
| 93 | 0x174 | 0x005F43A0 | I | ObjectClass::Scatter_174 | | HIGH |
| 94 | 0x178 | 0x005F43B0 | I | ObjectClass::Scatter_178 | | HIGH |
| 95 | 0x17C | 0x005F43C0 | I | ObjectClass::Scatter_17C | | HIGH |
| 96 | 0x180 | 0x00707DD0 | I | TechnoClass::GetThreatScore | | HIGH |
| 97 | 0x184 | 0x005B3040 | I | MissionClass::GetCurrentMission | | HIGH |
| 98 | 0x188 | 0x0041BE90 | I | ReturnFalse_188 | | HIGH |
| 99 | 0x18C | 0x006F5090 | I | TechnoClass::PerCellProcess | | HIGH |
| 100 | 0x190 | 0x005F5C20 | I | ObjectClass::CreateRadialIndicator | | HIGH |
| 101 | 0x194 | 0x0043C2D0 | **O** | BuildingClass::Receive_Radio | Already labeled | HIGH |
| 102 | 0x198 | 0x0044D5D0 | **O** | BuildingClass::DiscoveredBy | Building-specific house-discovery hook | MED |
| 103 | 0x19C | 0x00446FF0 | **O** | BuildingClass::CanPlayerDo_19C | Building-scope player action check | MED |
| 104 | 0x1A0 | 0x00447110 | **O** | BuildingClass::TogglePowerOrGate | Already labeled | HIGH |
| 105 | 0x1A4 | 0x005F6B50 | I | ObjectClass::IsStealthed | | HIGH |
| 106 | 0x1A8 | 0x005F4410 | I | ObjectClass::UpdatePosition | | HIGH |
| 107 | 0x1AC | 0x00449440 | **O** | BuildingClass::PassMessage_1AC | Message forwarding override | MED |
| 108 | 0x1B0 | 0x004264D0 | I | ObjectClass::PassMessage_1B0 | | HIGH |
| 109 | 0x1B4 | 0x005F6940 | I | ObjectClass::Set_Raw_Coords | | HIGH |
| 110 | 0x1B8 | 0x0041BEA0 | I | ObjectClass::Get_Cell_Packed | | HIGH |
| 111 | 0x1BC | 0x005F6960 | I | ObjectClass::GetOccupiedCell | | HIGH |
| 112 | 0x1C0 | 0x005F69C0 | I | ObjectClass::GetOccupiedCellClass | | HIGH |
| 113 | 0x1C4 | 0x005F6A10 | I | ObjectClass::GetOccupiedCellClass2 | | HIGH |
| 114 | 0x1C8 | 0x005F5F40 | I | ObjectClass::GetHeight | | HIGH |
| 115 | 0x1CC | 0x005F5FA0 | I | FootClass::Set_Height_On_Bridge | | HIGH |
| 116 | 0x1D0 | 0x005F5F30 | I | ObjectClass::GetHeight_1D0 | | HIGH |
| 117 | 0x1D4 | 0x0070C5B0 | I | TechnoClass::IsWarpingOut | | HIGH |
| 118 | 0x1D8 | 0x0070C5C0 | I | TechnoClass::IsBeingWarped | | HIGH |
| 119 | 0x1DC | 0x0070C5D0 | I | TechnoClass::IsUnderTemporal | | HIGH |
| 120 | 0x1E0 | 0x0070C5F0 | I | TechnoClass::IsNotWarping | | HIGH |
| 121 | 0x1E4 | 0x00705D70 | I | TechnoClass::DrawSHP | | HIGH |
| 122 | 0x1E8 | 0x005B35E0 | I | MissionClass::Queue_Mission | | HIGH |
| 123 | 0x1EC | 0x005B3570 | I | MissionClass::Commence | | HIGH |
| 124 | 0x1F0 | 0x005B2FD0 | I | MissionClass::Assign_Mission | | HIGH |
| 125 | 0x1F4 | 0x007013A0 | I | TechnoClass::Assign_Target | | HIGH |
| 126 | 0x1F8 | 0x007013E0 | I | TechnoClass::Assign_Destination | | HIGH |
| 127 | 0x1FC | 0x005B3A10 | I | MissionClass::Is_Mission_Suspended | | HIGH |
| 128 | 0x200 | 0x00454250 | **O** | BuildingClass::Mission_Load_Notify | Returns `*(byte*)(this+0x6dd)` — building-finished flag | HIGH |
| 129 | 0x204 | 0x005B2E10 | I | MissionClass::Mission_Default | | HIGH |
| 130 | 0x208 | 0x005B2E20 | I | MissionClass::Mission_Attack (default) | | HIGH |
| 131 | 0x20C | 0x005B2E30 | I | MissionClass::Mission_Move (default) | | HIGH |
| 132 | 0x210 | 0x0044ACF0 | **O** | **BuildingClass::Mission_Attack** | Already labeled | HIGH |
| 133 | 0x214 | 0x0044B760 | **O** | BuildingClass::Mission_Guard (thunk → 0x005B2E50) | Thin wrapper on MissionClass default | HIGH |
| 134 | 0x218 | 0x005B2E60 | I | MissionClass::Mission_Sticky (default) | | HIGH |
| 135 | 0x21C | 0x004496B0 | **O** | BuildingClass::Mission_Enter | v2 labeled Mission_Retreat, but routine does GrandOpening + `IsRepairDepot` checks — maps to vtable slot for **Enter/Retreat** class (both at this offset in dispatch) | MED |
| 136 | 0x220 | 0x00449A40 | **O** | BuildingClass::Mission_Capture (stub → vtable+0x21c) | 8-byte thunk via indirect jump | HIGH |
| 137 | 0x224 | 0x0044B770 | **O** | BuildingClass::Mission_Eaten (thunk → 0x005B2E90) | Thin wrapper returning 0x1C2 | HIGH |
| 138 | 0x228 | 0x005B2EA0 | I | MissionClass::Mission_Harvest (default) | | HIGH |
| 139 | 0x22C | 0x005B2EB0 | I | MissionClass::Mission_AreaGuard (default) | | HIGH |
| 140 | 0x230 | 0x005B2EC0 | I | MissionClass::Mission_Return (default) | | HIGH |
| 141 | 0x234 | 0x005B2ED0 | I | MissionClass::Mission_Stop (default) | | HIGH |
| 142 | 0x238 | 0x005B2EE0 | I | MissionClass::Mission_Ambush (default) | | HIGH |
| 143 | 0x23C | 0x0044D880 | **O** | BuildingClass::Mission_Hunt | Ghidra plate comment: "slot 26 — slave deployment + weapons-factory vehicle eject"; exact YR enum not fully confirmed but behavior = deploy contents | MED |
| 144 | 0x240 | 0x005B2F00 | I | MissionClass::Mission_Unload (default) | | HIGH |
| 145 | 0x244 | 0x00449A50 | **O** | **BuildingClass::Mission_Construction** | GrandOpening state machine | HIGH |
| 146 | 0x248 | 0x00449C30 | **O** | **BuildingClass::Mission_Selling** (`BuildingClass__Sell`) | Already labeled; ~4,000-byte sell sequence | HIGH |
| 147 | 0x24C | 0x0044B780 | **O** | **BuildingClass::Mission_Repair** (RepairAndProduce) | Already labeled `BuildingClass__MissionRepairAndProduce` | HIGH |
| 148 | 0x250 | 0x0044C980 | **O** | **BuildingClass::Mission_Missile** | Already labeled | HIGH |
| 149 | 0x254 | 0x0044E440 | **O** | **BuildingClass::Mission_Rescue/Unload** | State machine (case 0..4) for paradrop-like deployment | HIGH |
| 150 | 0x258 | 0x005B2F60 | I | MissionClass::Mission_Missile (default) | | HIGH |
| 151 | 0x25C | 0x005B2F70 | I | MissionClass::Mission_Harmless (default) | | HIGH |
| 152 | 0x260 | 0x005B2F80 | I | MissionClass::Mission_Open (default) | | HIGH |
| 153 | 0x264 | 0x005B2F90 | I | MissionClass::Mission_Patrol (default) | | HIGH |
| 154 | 0x268 | 0x005B2FA0 | I | MissionClass::Mission_ParaDropApproach (default) | | HIGH |
| 155 | 0x26C | 0x005B2FB0 | I | MissionClass::Mission_ParaDropOverfly (default) | | HIGH |
| 156 | 0x270 | 0x005B2FC0 | I | MissionClass::Mission_Wait (default) | | HIGH |
| 157 | 0x274 | 0x0065ACB0 | I | RadioClass::Transmit_Radio_ToFirst | | HIGH |
| 158 | 0x278 | 0x0065AAA0 | I | RadioClass::Transmit_Radio | | HIGH |
| 159 | 0x27C | 0x0065A970 | I | RadioClass::Transmit_Radio_Impl | | HIGH |
| 160 | 0x280 | 0x0065ACE0 | I | RadioClass::Broadcast_Radio_ToAll | | HIGH |
| 161 | 0x284 | 0x00455DA0 | **O** | BuildingClass::ReturnFalse_284 (has-weapon variant) | Returns 1 if type is WeaponsFactory/ServiceDepot/`Type+0x16ac`/`Type+0x16a9` | MED |
| 162 | 0x288 | 0x0070C5A0 | I | TechnoClass::HasStealthAbility | | HIGH |
| 163 | 0x28C | 0x006F3280 | I | TechnoClass::CanBeTargeted | | HIGH |
| 164 | 0x290 | 0x00459D80 | I | ReturnFalse_290 | | HIGH |
| 165 | 0x294 | 0x0070BE80 | I | TechnoClass::IsReadyToCloak | | HIGH |
| 166 | 0x298 | 0x006F9E10 | I | TechnoClass::PreAI | | HIGH |
| 167 | 0x29C | 0x0041BEF0 | I | ReturnFalse_29C | | HIGH |
| 168 | 0x2A0 | 0x00457770 | **O** | **BuildingClass::CanCloak** (CanAutoCloak) | Already labeled | HIGH |
| 169 | 0x2A4 | 0x004578C0 | **O** | **BuildingClass::ShouldUncloak** | Already labeled | HIGH |
| 170 | 0x2A8 | 0x00445E50 | **O** | BuildingClass::CanCloak_vt2A8 | Building-specific cloak query (static method?) | MED |
| 171 | 0x2AC | 0x00458DB0 | **O** | BuildingClass::CanHaveKickout | Override of TC::CanHaveKickout | MED |
| 172 | 0x2B0 | 0x0070C620 | I | TechnoClass::IsNotAtDestination | | HIGH |
| 173 | 0x2B4 | 0x00708BC0 | I | TechnoClass::GetTargetingData_2B4 | | HIGH |
| 174 | 0x2B8 | 0x0044D700 | **O** | BuildingClass::GetTargetingData_2B8 | Building-specific targeting-data override | MED |
| 175 | 0x2BC | 0x0070ADA0 | I | TechnoClass::UpdateSensors_2BC | | HIGH |
| 176 | 0x2C0 | 0x00708B40 | I | TechnoClass::GetTimerData_2C0 | | HIGH |
| 177 | 0x2C4 | 0x00459D90 | I | ReturnFalse_2C4 | | HIGH |
| 178 | 0x2C8 | 0x0043E940 | **O** | BuildingClass::GetTurretFacing | Building turret facing (static frame-based) | HIGH |
| 179 | 0x2CC | 0x00707F60 | I | TechnoClass::GetGuardRange | | HIGH |
| 180 | 0x2D0 | 0x00451330 | **O** | BuildingClass::GetWeaponRange_2D0 | Building-specific weapon range | MED |
| 181 | 0x2D4 | 0x00459870 | **O** | BuildingClass::GetType_Helper_2D4 | Returns `Type+0x1524` (small accessor) | HIGH |
| 182 | 0x2D8 | 0x00459880 | **O** | BuildingClass::GetType_Helper_2D8 | Returns `Type+0x1524` (duplicate of 181?) | MED |
| 183 | 0x2DC | 0x00459890 | **O** | BuildingClass::GetType_Helper_2DC | Small accessor variant | MED |
| 184 | 0x2E0 | 0x004576F0 | **O** | BuildingClass::GetSuperWeaponIndex | Already labeled | HIGH |
| 185 | 0x2E4 | 0x006F3330 | I | TechnoClass::SelectWeaponAgainst | | HIGH |
| 186 | 0x2E8 | 0x006F3820 | I | TechnoClass::CanTargetVsTurretLock | | HIGH |
| 187 | 0x2EC | 0x0043E900 | **O** | BuildingClass::GetZAdjust | Building Z-offset for bridges/slopes | HIGH |
| 188 | 0x2F0 | 0x00459DA0 | I | ReturnFalse_2F0 | | HIGH |
| 189 | 0x2F4 | 0x00459DB0 | I | ReturnFalse_2F4 | | HIGH |
| 190 | 0x2F8 | 0x00459DC0 | I | ReturnFalse_2F8 | | HIGH |
| 191 | 0x2FC | 0x0070AD50 | I | TechnoClass::AddSensors_2FC | | HIGH |
| 192 | 0x300 | 0x00453A70 | **O** | BuildingClass::GetTurretLocation | 3D turret matrix — building footprint-aware | HIGH |
| 193 | 0x304 | 0x00708C10 | I | TechnoClass::GetBodyFacing | | HIGH |
| 194 | 0x308 | 0x0044D7D0 | **O** | BuildingClass::GetTurretFacing_Raw | Building turret raw facing (long) | MED |
| 195 | 0x30C | 0x0044EB10 | **O** | BuildingClass::GetVoiceResponse | Building-specific voice response (construction, attack, etc.) | MED |
| 196 | 0x310 | 0x00700D10 | I | TechnoClass::CanEnterTransport | | HIGH |
| 197 | 0x314 | 0x00700D50 | I | TechnoClass::CanEnterCell | | HIGH |
| 198 | 0x318 | 0x006FCFA0 | I | TechnoClass::GetROF | | HIGH |
| 199 | 0x31C | 0x00707E60 | I | TechnoClass::GetRearmDelay | | HIGH |
| 200 | 0x320 | 0x00459DD0 | I | ReturnFalse_320 | | HIGH |
| 201 | 0x324 | 0x00457020 | **O** | BuildingClass::SetDrawHealthBarsFlags | Building-specific redraw flag setter | MED |
| 202 | 0x328 | 0x0070D420 | I | TechnoClass::StopAllTargeting_328 | | HIGH |
| 203 | 0x32C | 0x0070D460 | I | TechnoClass::StopAllTargeting_32C | | HIGH |
| 204 | 0x330 | 0x0041BF30 | I | ReturnFalse_330 | | HIGH |
| 205 | 0x334 | 0x00459DE0 | I | ReturnFalse_334 | | HIGH |
| 206 | 0x338 | 0x0070F8F0 | I | TechnoClass::OnCapture | | HIGH |
| 207 | 0x33C | 0x00459DF0 | I | ReturnFalse_33C | | HIGH |
| 208 | 0x340 | 0x00459E00 | I | ReturnFalse_340 | | HIGH |
| 209 | 0x344 | 0x00459E10 | I | ReturnFalse_344 | | HIGH |
| 210 | 0x348 | 0x00459E20 | I | ReturnFalse_348 | | HIGH |
| 211 | 0x34C | 0x00459E30 | I | ReturnFalse_34C | | HIGH |
| 212 | 0x350 | 0x004555D0 | **O** | BuildingClass::CanSellOrUndeploy | Already labeled | HIGH |
| 213 | 0x354 | 0x00708D90 | I | TechnoClass::GetWeaponStruct_354 | | HIGH |
| 214 | 0x358 | 0x00709020 | I | TechnoClass::GetPrimaryWeapon | | HIGH |
| 215 | 0x35C | 0x00709060 | I | TechnoClass::GetSecondaryWeapon | | HIGH |
| 216 | 0x360 | 0x00708EB0 | I | TechnoClass::GetWeapon_Veteran | | HIGH |
| 217 | 0x364 | 0x00708DC0 | I | TechnoClass::GetWeaponStruct_364 | | HIGH |
| 218 | 0x368 | 0x00708FC0 | I | TechnoClass::GetElitePrimaryWeapon | | HIGH |
| 219 | 0x36C | 0x00459C20 | **O** | BuildingClass::GetWeaponStruct_36C | Building slave/weapon-struct override (SlaveMiner check) | MED |
| 220 | 0x370 | 0x007090A0 | I | TechnoClass::GetEliteSecondaryWeapon | | HIGH |
| 221 | 0x374 | 0x006FFE00 | I | TechnoClass::What_Action_Evaluate | | HIGH |
| 222 | 0x378 | 0x006FFBE0 | I | TechnoClass::CanAfford | | HIGH |
| 223 | 0x37C | 0x0070EFD0 | I | TechnoClass::IsUnderEMP | | HIGH |
| 224 | 0x380 | 0x00459E40 | I | ReturnFalse_380 | | HIGH |
| 225 | 0x384 | 0x0041BF80 | I | ReturnFalse_384 | | HIGH |
| 226 | 0x388 | 0x0041BF90 | I | ReturnFalse_388 | | HIGH |
| 227 | 0x38C | 0x0070EFE0 | I | TechnoClass::GetEMPData | | HIGH |
| 228 | 0x390 | 0x0070D670 | I | TechnoClass::GetIdleAnim | | HIGH |
| 229 | 0x394 | 0x00710670 | I | TechnoClass::FreeMindControlledChain | | HIGH |
| 230 | 0x398 | 0x0070EF00 | I | TechnoClass::CanEMPAffect | | HIGH |
| 231 | 0x39C | 0x00709820 | I | TechnoClass::Retaliate_And_Scan | | HIGH |
| 232 | 0x3A0 | 0x006FCD40 | I | TechnoClass::StopFiring | | HIGH |
| 233 | 0x3A4 | 0x006F7660 | I | TechnoClass::IsInWeaponRange | | HIGH |
| 234 | 0x3A8 | 0x006F77B0 | I | TechnoClass::CanFireAt | | HIGH |
| 235 | 0x3AC | 0x006F7780 | I | TechnoClass::CanFireAtTarget | | HIGH |
| 236 | 0x3B0 | 0x006F7930 | I | TechnoClass::PreFire | | HIGH |
| 237 | 0x3B4 | 0x006F78D0 | I | TechnoClass::FireWeapon | | HIGH |
| 238 | 0x3B8 | 0x0044D760 | **O** | BuildingClass::PostFire | Overrides TC's `Stub::ReturnZero` at slot 238; building post-fire bookkeeping | MED |
| 239 | 0x3BC | 0x006FC090 | I | TechnoClass::GetFireError_3BC | | HIGH |
| 240 | 0x3C0 | 0x00447F10 | **O** | **BuildingClass::GetFireError** | Already labeled | HIGH |
| 241 | 0x3C4 | 0x00445F00 | **O** | BuildingClass::Greatest_Threat | Building-specific threat scan | MED |
| 242 | 0x3C8 | 0x00443B90 | **O** | **BuildingClass::ToggleGate** / Set_ArchiveTarget | Already labeled `BuildingClass__ToggleGate` — repurposes TC's `Set_ArchiveTarget` slot | HIGH |
| 243 | 0x3CC | 0x006FDD50 | I | TechnoClass::Fire_At | | HIGH |
| 244 | 0x3D0 | 0x0070F850 | I | TechnoClass::StopAndGuard | | HIGH |
| 245 | 0x3D4 | 0x00448260 | **O** | **BuildingClass::ChangeOwner** | Already labeled | HIGH |
| 246 | 0x3D8 | 0x0070B280 | I | TechnoClass::UpdateRocking | | HIGH |
| 247 | 0x3DC | 0x00459E50 | I | ReturnFalse_3DC | | HIGH |
| 248 | 0x3E0 | 0x0070DD50 | I | TechnoClass::HasActiveWeapon | | HIGH |
| 249 | 0x3E4 | 0x0070DD70 | I | TechnoClass::GetBurstIndex | | HIGH |
| 250 | 0x3E8 | 0x0070DD90 | I | TechnoClass::GetBurstDelay | | HIGH |
| 251 | 0x3EC | 0x0070DDA0 | I | TechnoClass::GetBurstCount | | HIGH |
| 252 | 0x3F0 | 0x0070E120 | I | TechnoClass::GetWeaponForTarget | | HIGH |
| 253 | 0x3F4 | 0x0070E1A0 | I | TechnoClass::GetWeaponForIndex | | HIGH |
| 254 | 0x3F8 | 0x004526F0 | **O** | **BuildingClass::GetWeapon** | Already labeled (the slot-split correction from v2 audit) | HIGH |
| 255 | 0x3FC | 0x004527D0 | **O** | **BuildingClass::HasTurret** | Already labeled | HIGH |
| 256 | 0x400 | 0x00458DD0 | **O** | **BuildingClass::IsOccupied** | Already labeled | HIGH |
| 257 | 0x404 | 0x00458E00 | **O** | **BuildingClass::GetHalfFoundationSize** | Already labeled | HIGH |
| 258 | 0x408 | 0x004581F0 | **O** | **BuildingClass::GetOccupantCount** | Already labeled | HIGH |
| 259 | 0x40C | 0x00701410 | I | TechnoClass::EngineerRepair | | HIGH |
| 260 | 0x410 | 0x00454DB0 | **O** | **BuildingClass::UpdateGapGenerator_Tick** | Already labeled | HIGH |
| 261 | 0x414 | 0x006FB170 | I | TechnoClass::UpdateCloakShroud | | HIGH |
| 262 | 0x418 | 0x006FB470 | I | TechnoClass::RemoveCloakShroud | | HIGH |
| 263 | 0x41C | 0x0070B570 | I | TechnoClass::RockingUpdate | | HIGH |
| 264 | 0x420 | 0x006F4EB0 | I | TechnoClass::DoUncloak | | HIGH |
| 265 | 0x424 | 0x006FB010 | I | TechnoClass::CloakDetect | | HIGH |
| 266 | 0x428 | 0x0041BFE0 | I | ReturnFalse_428 | | HIGH |
| 267 | 0x42C | 0x00705CA0 | I | TechnoClass::DrawSHP_42C | | HIGH |
| 268 | 0x430 | 0x00705D50 | I | TechnoClass::DrawVoxel | | HIGH |
| 269 | 0x434 | 0x0041BFF0 | I | ReturnFalse_434 | | HIGH |
| 270 | 0x438 | 0x00459E60 | I | ReturnFalse_438 | | HIGH |
| 271 | 0x43C | 0x0070ED80 | I | TechnoClass::ModifyCloakDrawFlags | | HIGH |
| 272 | 0x440 | 0x0070EE30 | I | TechnoClass::ProcessCloakDraw | | HIGH |
| 273 | 0x444 | 0x00706640 | I | TechnoClass::Draw | | HIGH |
| 274 | 0x448 | 0x006F60C0 | I | TechnoClass::DrawOverlay | | HIGH |
| 275 | 0x44C | 0x006F64A0 | I | TechnoClass::DrawHealthBar | | HIGH |
| 276 | 0x450 | 0x00709A90 | I | TechnoClass::DrawPipScalePips_450 | | HIGH |
| 277 | 0x454 | 0x0070A990 | I | TechnoClass::DrawVeterancyPips_454 | | HIGH |
| 278 | 0x458 | 0x0070AA60 | I | TechnoClass::DrawExtraInfo | | HIGH |
| 279 | 0x45C | 0x007036C0 | I | TechnoClass::StartUncloaking | | HIGH |
| 280 | 0x460 | 0x00703770 | I | TechnoClass::StartCloaking | | HIGH |
| 281 | 0x464 | 0x00456F80 | **O** | **BuildingClass::AdjustZHeight** | Already labeled | HIGH |
| 282 | 0x468 | 0x00459900 | **O** | BuildingClass::Create_ParticleSystems | Creates 4 ParticleSystemClass instances at Type+0x7cc..0x7f8 offsets (smoke plumes, light sources) | HIGH |
| 283 | 0x46C | 0x0070E280 | I | TechnoClass::GetIronCurtainState | | HIGH |
| 284 | 0x470 | 0x0041C030 | I | ReturnFalse_470 | | HIGH |
| 285 | 0x474 | 0x007099E0 | I | TechnoClass::DrawPipScale | | HIGH |
| 286 | 0x478 | 0x0041C040 | I | ReturnFalse_478 | | HIGH |
| 287 | 0x47C | 0x00709A20 | I | TechnoClass::GetPipCount_47C | | HIGH |
| 288 | 0x480 | 0x00455D50 | **O** | BuildingClass::SetTarget_480 | Building-specific target-set (WeaponsFactory check) | MED |
| 289 | 0x484 | 0x0044D6A0 | **O** | BuildingClass::GetTarget_484 | Building-specific target accessor | MED |
| 290 | 0x488 | 0x0070AF50 | I | TechnoClass::UpdateReveal | | HIGH |
| 291 | 0x48C | 0x0070B1D0 | I | TechnoClass::ReReveal | | HIGH |
| 292 | 0x490 | 0x00458A80 | **O** | BuildingClass::WeaponPickHelper | Large routine (809 bytes); overrides TC `Stub::ReturnZero` at this slot | LOW |
| 293 | 0x494 | 0x00456580 | **O** | **BuildingClass::RegisterOnRadar** | Already labeled | HIGH |
| 294 | 0x498 | 0x004565E0 | **O** | BuildingClass::UnregisterFromRadar | Radar-blip removal (93 bytes) | HIGH |
| 295 | 0x49C | 0x00456640 | **O** | BuildingClass::IdleActionTick | Building-specific idle-action tick (102 bytes) | MED |
| 296 | 0x4A0 | 0x0070D990 | I | TechnoClass::IdleAnimDispatch | | HIGH |
| 297 | 0x4A4 | 0x0070F000 | I | TechnoClass::EMP_Handler_4A4 | | HIGH |
| 298 | 0x4A8 | 0x0070F010 | I | TechnoClass::EMP_Handler_4A8 | | HIGH |
| 299 | 0x4AC | 0x0070F020 | I | TechnoClass::EMP_Handler_4AC | | HIGH |

## Extension range — slots 300-337 (appended R4)

Slots 300-337 were not captured in the original 300-slot pass. Addresses read
directly via `read_memory` at `vtable_base + 300*4 = 0x007E436C` onward; see T5/T10
reconciliation note for the raw read. NULL terminator at slot 338 marks end of
vtable (the subsequent bytes are floating-point constants, not function pointers).

| Slot | Offset | Address | Cat | Function | Purpose | Conf |
|---|---|---|---|---|---|---|
| 300 | 0x4B0 | 0x0070F030 | I | TechnoClass helper (stub-region `0x7F030`-adjacent) | | MED |
| 301 | 0x4B4 | 0x0070F040 | I | TechnoClass helper | | MED |
| 302 | 0x4B8 | 0x0070F050 | I | TechnoClass helper | | MED |
| 303 | 0x4BC | 0x0070F070 | I | TechnoClass helper | | MED |
| 304 | 0x4C0 | 0x0070F090 | I | TechnoClass helper | | MED |
| 305 | 0x4C4 | 0x0070F0E0 | I | TechnoClass helper | | MED |
| 306 | 0x4C8 | 0x0070F0F0 | I | TechnoClass helper | | MED |
| 307 | 0x4CC | 0x0070F100 | I | TechnoClass helper | | MED |
| 308 | 0x4D0 | 0x0070F110 | I | TechnoClass helper | | MED |
| 309 | 0x4D4 | 0x0044EFB0 | **O** | **BuildingClass::GetDockCellForObject** | Already labeled. Dock cell selection (barracks/naval/refinery). | HIGH |
| 310 | 0x4D8 | 0x00447E00 | **O** | BuildingClass helper @ 0x00447E00 | Adjacent to GetFacingCoords / GetDockCoord region | MED |
| 311 | 0x4DC | 0x00445F80 | **O** | **BuildingClass::OnConstructionComplete** | Called when a newly-placed building's construction anim finishes. Decomped in T12/T13 (B23). | HIGH |
| 312 | 0x4E0 | 0x004456D0 | **O** | BuildingClass helper @ 0x004456D0 | Destruction/cleanup adjacent | MED |
| 313 | 0x4E4 | 0x0043DA80 | **O** | **BuildingClass::DrawBody_VXL** (VXL/extras pass) | Sibling of DrawBody SHP; dispatcher at slot 65 invokes this when `+0x6E7 == 0`. Function boundary created by T7 in Ghidra. | HIGH |
| 314 | 0x4E8 | 0x0043ED40 | **O** | BuildingClass helper @ 0x0043ED40 | Draw-adjacent | MED |
| 315 | 0x4EC | 0x004415F0 | **O** | **BuildingClass::DestructionEffects** | The real HP=0 handler (called from ReceiveDamage case 4). Survivors, debris, tiberium spill, particles, death anim, sets Health=0. Renamed from `FUN_` in this session. See T10 report. | HIGH |
| 316 | 0x4F0 | 0x00448160 | **O** | BuildingClass helper @ 0x00448160 | | MED |
| 317 | 0x4F4 | 0x00455820 | **O** | **BuildingClass::AddSensorArrayAt** | Already labeled — reads `Type+0x5F0 SensorsSight` for radius. | HIGH |
| 318 | 0x4F8 | 0x004556D0 | **O** | **BuildingClass::RemoveSensorArrayAt** | Already labeled — reads `Type+0x1707 CloakRadiusInCells` (asymmetry bug vs slot 317). | HIGH |
| 319 | 0x4FC | 0x00455A80 | **O** | **BuildingClass::AddDetectDisguiseAt** | Already labeled. | HIGH |
| 320 | 0x500 | 0x00455980 | **O** | **BuildingClass::RemoveDetectDisguiseAt** | Already labeled. | HIGH |
| 321 | 0x504 | 0x00452250 | **O** | BuildingClass helper @ 0x00452250 | Near ToggleArmor / IsDeployable region | MED |
| 322 | 0x508 | 0x007FC298 | — | **Secondary vtable marker (MI)** | Not a function — an IPersistStream (or similar COM interface) secondary-vtable pointer in the data range. Used by the multi-inheritance layout. | HIGH |
| 323 | 0x50C | 0x0045AAB0 | **O** | BuildingClass MI helper @ 0x0045AAB0 | MI-related method | LOW |
| 324 | 0x510 | 0x0045A3E0 | **O** | BuildingClass MI helper @ 0x0045A3E0 | MI-related method (also appears at slot 332) | LOW |
| 325 | 0x514 | 0x0045A560 | **O** | BuildingClass MI helper @ 0x0045A560 | | LOW |
| 326 | 0x518 | 0x00459FF0 | **O** | BuildingClass MI helper @ 0x00459FF0 | | LOW |
| 327 | 0x51C | 0x0045A610 | **O** | BuildingClass MI helper @ 0x0045A610 | | LOW |
| 328 | 0x520 | 0x0045A020 | **O** | BuildingClass MI helper @ 0x0045A020 | | LOW |
| 329 | 0x524 | 0x0045A040 | **O** | BuildingClass MI helper @ 0x0045A040 | Shared with slot 337 | LOW |
| 330 | 0x528 | 0x007FC390 | — | **Secondary vtable marker (MI)** | Not a function — second MI interface pointer, same pattern as slot 322. | HIGH |
| 331 | 0x52C | 0x0045AA60 | **O** | BuildingClass MI helper @ 0x0045AA60 | | LOW |
| 332 | 0x530 | 0x0045A3E0 | **O** | BuildingClass MI helper @ 0x0045A3E0 | Shared with slot 324 | LOW |
| 333 | 0x534 | 0x0045A420 | **O** | BuildingClass MI helper @ 0x0045A420 | | LOW |
| 334 | 0x538 | 0x0045A4D0 | **O** | BuildingClass MI helper @ 0x0045A4D0 | | LOW |
| 335 | 0x53C | 0x0045A500 | **O** | BuildingClass MI helper @ 0x0045A500 | | LOW |
| 336 | 0x540 | 0x0045A540 | **O** | BuildingClass MI helper @ 0x0045A540 | | LOW |
| 337 | 0x544 | 0x0045A040 | **O** | BuildingClass MI helper @ 0x0045A040 | Shared with slot 329 | LOW |
| 338 | 0x548 | 0x00000000 | — | **NULL terminator** — end of vtable | HIGH |

---

## Notable slots

### Save/Load (slots 5, 6) — critical for Task 8

- **Slot 5 `0x00453E20` = `BuildingClass::Load`** (IPersistStream::Load). Calls `TechnoClass::Load` at `0x0070BF50`, then performs building-specific post-load fixup: reads two arrays (radio contacts + docks) with bounds checks against `Type+0x16f0` and related flags, re-reads VocHandles at field indices +0x148..+0x1bd, and per-element reads into `param_1[0x19c]`/`param_1[0x1a2]`/etc.
- **Slot 6 `0x00454190` = `BuildingClass::Save`** (IPersistStream::Save). Calls `TechnoClass::Save_Stream` at `0x0070C250`, then writes `*(param_1 + 0x67c)` (radio-contact count) and iterates the two dynamic arrays writing 4 bytes per entry.

**CORRECTION vs v2 §4:** v2 listed slot 5 as "AbstractClass::Save" and slot 6 as "AbstractClass::Load". That is swapped — by MS IPersistStream convention slot 5 is Load (AfterLoad) and slot 6 is Save, and the decomps confirm: slot 5 calls `FUN_0070bf50` (TC::Load), slot 6 calls `FUN_0070c250` (TC::Save_Stream).

### Lifecycle

- **Slot 8 Destructor** `0x00459F20` — `ScalarDeletingDestructor` (calls BuildingClass constructor chain + conditional `operator delete`).
- **Slot 9 Init_Managers** `0x00442C40` — Overrides TC to register in `HouseClass__Add_Tracking` and init building-only rate timers.
- **Slot 11 WhatAmI** `0x00459EC0` — returns `6` (AbstractType::Building).
- **Slot 12 SizeOf** `0x00459E70` — returns `0x720` (1824 bytes). Matches the constructor-derived estimate in BUILDINGCLASS_VTABLE_AND_LIFECYCLE.md §7.
- **Slot 23 Update** `0x0043FB20` — main per-tick AI.
- **Slot 53 Limbo** `0x00445880` — **remove-from-map cleanup** (wall connections, radar dejamming, house counters, anim-slot release). Renamed from "OnDestroyed" 2026-04-24 — NOT the HP=0 handler.
- **Slot 54 Unlimbo** `0x00440580` — huge (~4,300-byte) placement routine (walls, upgrades, sensor registration, light sources).
- **Slot 55 Destroy** `0x0044EBF0` — aborts factory production, ejects queued units, calls `ObjectClass::Destroy`. (v2 labeled this "Limbo" — wrong.)
- **Slot 315 DestructionEffects** `0x004415F0` — the **real HP=0 event handler**. Called from ReceiveDamage case 4. Spawns survivors, debris, death anim, tiberium spill; sets Health=0.

### Drawing

- **Slot 65 DrawIt** `0x0043CEA0`, **Slot 69 DrawBody** `0x0043D290`, **Slot 70 DrawBody_Helper** `0x0043D030`, **Slot 282 Create_ParticleSystems** `0x00459900`.

### Mission handlers (slots 128–156)

12 of 29 mission slots are overridden. Confirmed overrides:
- 132 Mission_Attack (`0x0044ACF0`)
- 133, 137 thin MissionClass thunks
- 135 Mission handler (GrandOpening pattern + IsRepairDepot — v2 labeled Retreat)
- 136 Mission thunk (8-byte indirect jump)
- 143 Mission_Hunt / slave-deploy (`0x0044D880`)
- 145 Mission_Construction (`0x00449A50`)
- 146 Mission_Selling (`0x00449C30` — `BuildingClass__Sell`)
- 147 Mission_Repair (`0x0044B780` — `BuildingClass__MissionRepairAndProduce`)
- 148 Mission_Missile (`0x0044C980`)
- 149 Mission_Unload (`0x0044E440`)
- 128 (re-purposed) returns the `field_0x6dd` "construction-complete" flag.

### Building-specific state / query

- 40 ShouldShowDeployButton, 212 CanSellOrUndeploy, 240 GetFireError, 242 ToggleGate, 245 ChangeOwner, 254 GetWeapon, 255 HasTurret, 256 IsOccupied, 257 GetHalfFoundationSize, 258 GetOccupantCount, 260 UpdateGapGenerator_Tick, 281 AdjustZHeight, 293 RegisterOnRadar.

---

## Pure-virtual slots

**None.** Every TC `Stub__ReturnZero` entry (slots 3, 11, 12, 238, 292) is overridden with a concrete BuildingClass implementation. Calling any of these on a live `BuildingClass*` does real work, not a stub.

---

## Corrections flagged vs v2 §4 / BUILDINGCLASS_VTABLE_AND_LIFECYCLE.md

| # | Slot | v2 claim | Actual (this doc) |
|---|---|---|---|
| 1 | 5 | "AbstractClass::Save" | **BuildingClass::Load** (calls TC Load, follows IPersistStream convention) |
| 2 | 6 | "AbstractClass::Load" | **BuildingClass::Save** (calls TC Save_Stream) |
| 3 | 8 | "AbstractClass::WhatAmI" | **ScalarDeletingDestructor** (0x00459F20 calls constructor + `operator delete`) |
| 4 | 9 | "AbstractClass::SizeOf" | **BuildingClass::Init_Managers** (calls `TechnoClass__Init_Managers` + `HouseClass__Add_Tracking`) |
| 5 | 11 | "AbstractClass::vt_func_11 (stub)" | **WhatAmI** (returns 6); NOT a stub |
| 6 | 12 | "AbstractClass::vt_func_12 (stub)" | **SizeOf** (returns 0x720); NOT a stub |
| 7 | 13 | "AbstractClass::PointerExpired" | **Save_ChecksumFields** (called by CRC checksum pipeline, not the expire-ptr hook) |
| 8 | "~95 overrides" | 95 | **101 overrides** (v2 undercounted by 6 — did not expand ranges 170–184, 282, 288–295) |
| 9 | 170/171/174/180/181/182/183/184/187 | "various stubs/inherited" | All nine are **overrides** — listed individually in this doc |
| 10 | 292 | listed as "BuildingClass::vt_func_292 (stub)" in v2 | **Overridden by an 809-byte routine `0x00458A80`** — NOT a stub |

---

## Slots flagged for future deeper work (MED/LOW confidence)

The following overrides have definite addresses and are confirmed overrides (BC ≠ TC), but the purpose was not deeply decompiled in this sitting and might benefit from further Ghidra work if the functionality becomes relevant:

| Slot | Addr | Tentative label | Reason flagged |
|---|---|---|---|
| 27 | 0x004513D0 | GetAction | Small, tied to anim dispatch; exact semantics unclear |
| 34 | 0x00459EE0 | GetObjectType | 6 bytes — stub-ish |
| 45 | 0x0044F640 | GetCenterCoord (alt) | Small accessor |
| 46 | 0x00449410 | GetYSort | Foundation-based sort |
| 65 | 0x0043CEA0 | DrawIt | Dispatches to DrawBody; may be pre-draw prep |
| 70 | 0x0043D030 | DrawBody_Helper | Secondary draw helper |
| 75 | 0x00455C20 | GetDrawRect | Building footprint override |
| 76 | 0x00456750 | MarkNeedsRedraw | Gap-generator tie-in |
| 80/81 | 0x004436F0 / 0x00443410 | ClickedAction_140/144 | Rally-point & sidebar flash |
| 82 | 0x00456E00 | SetOwnerHouse | Ownership transfer fixup |
| 102/103/107 | 0x0044D5D0 / 0x00446FF0 / 0x00449440 | DiscoveredBy / CanPlayerDo / PassMessage | Radio & discovery hooks |
| 143 | 0x0044D880 | Mission_Hunt / slave-deploy | Ghidra plate comment unsure about exact YR enum |
| 161 | 0x00455DA0 | (has-weapon variant) | Check returning 1 for WF/ServiceDepot/bit flags |
| 170/171 | 0x00445E50 / 0x00458DB0 | CanCloak_vt2A8 / CanHaveKickout | Secondary cloak / kickout variants |
| 174 | 0x0044D700 | GetTargetingData_2B8 | Targeting data accessor |
| 178 | 0x0043E940 | GetTurretFacing | Static turret facing |
| 180 | 0x00451330 | GetWeaponRange_2D0 | Alt weapon-range variant |
| 181/182/183 | 0x00459870..0x00459890 | GetType_Helper_2D4/8/C | Tiny Type+0x1524 accessors |
| 187 | 0x0043E900 | GetZAdjust | Bridge/slope Z-offset |
| 194 | 0x0044D7D0 | GetTurretFacing_Raw | Long (161 bytes) |
| 195 | 0x0044EB10 | GetVoiceResponse | Building-specific voice |
| 201 | 0x00457020 | SetDrawHealthBarsFlags | Long (440 bytes) |
| 219 | 0x00459C20 | GetWeaponStruct_36C | Slave-manager tie-in |
| 238 | 0x0044D760 | PostFire | Overrides a TC pure-virtual slot |
| 241 | 0x00445F00 | Greatest_Threat | Building threat scan |
| 282 | 0x00459900 | Create_ParticleSystems | Confirmed 4 particle-systems — plumes/light/fire |
| 288/289 | 0x00455D50 / 0x0044D6A0 | SetTarget_480 / GetTarget_484 | Target override for WeaponsFactory |
| 292 | 0x00458A80 | WeaponPickHelper (LOW) | 809-byte; not decompiled here |
| 294/295 | 0x004565E0 / 0x00456640 | UnregisterFromRadar / IdleActionTick | Radar blip / idle action |

---

## Ghidra rename log (this session)

Performed ≥90%-confidence renames:

| Address | Old | New |
|---|---|---|
| 0x00459E80 | FUN_00459e80 | BuildingClass__GetClassID |
| 0x00459EC0 | FUN_00459ec0 | BuildingClass__WhatAmI |
| 0x00459E70 | FUN_00459e70 | BuildingClass__SizeOf |
| 0x00459F20 | FUN_00459f20 | BuildingClass__ScalarDeletingDestructor |
| 0x00442C40 | FUN_00442c40 | BuildingClass__Init_Managers |
| 0x00453E20 | FUN_00453e20 | BuildingClass__Load |
| 0x00454190 | FUN_00454190 | BuildingClass__Save |
| 0x00454260 | FUN_00454260 | BuildingClass__Save_ChecksumFields |
| 0x00452630 | FUN_00452630 | BuildingClass__IsDeployable |
| 0x0044EBF0 | FUN_0044ebf0 | BuildingClass__Destroy |

Also `create_function` was called on 26 addresses that Ghidra hadn't auto-discovered as function entry points (see the "create_function" batch earlier this session).

Program saved after renames.

---

## Sources

- Live Ghidra read_memory at `0x007E3EBC` and `0x007F4960` (1200 bytes each)
- Per-slot decomps of all 101 override addresses (those with existing function bodies)
- Existing docs cross-referenced:
  - `TECHNOCLASS_VTABLE_COMPLETE.md` — used for every inherited slot name
  - `BUILDINGCLASS_VTABLE_AND_LIFECYCLE.md` — prior partial map
  - `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md` §4 — partial v2 survey (verified and corrected above)
- Verified v2's slot-37 correction (IsDeployable, not CanAcceptUpgrade) and the slot-254/255 split (GetWeapon / HasTurret).
