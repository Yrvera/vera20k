# BuildingClass Vtable & Lifecycle — Ghidra Report

**Date:** 2026-04-06  
**Binary:** gamemd.exe  
**Confidence:** HIGH — all addresses verified from binary vtable data and decompilation  

---

## 1. Vtable Location

- **Primary vtable:** `0x007E3EBC` (label: `vtable_BuildingClass`)
- **Secondary vtables (MI):**
  - `vtable__BuildingClass__secondary_4` @ `0x007E3EA0`
  - `vtable__BuildingClass__secondary_8` @ `0x007E3E98`
  - `vtable__BuildingClass__secondary_12` @ `0x007E3E90`
- **RTTI type descriptor string:** `0x00818D68` — `.?AVBuildingClass@@`
- **Total vtable slots:** ~360 (corrected 2026-05-29: was "300 (slots 0–299, offsets 0x000–0x4AC)"; binary vtable continues past 0x007E436C with valid function pointers; first non-pointer value appears near 0x007E4464 (IEEE float 0x3FC00000), giving ~360 slots — OPERATOR_OR_ORDER_DRIFT via read_memory 0x007E436C, 0x007E4450)

---

## 2. Inheritance Chain

```
IUnknown (COM)
  └─ AbstractClass          vtable @ 0x007E1F50
       └─ ObjectClass       vtable @ 0x007EF060
            └─ MissionClass  vtable @ 0x007EDCC0
                 └─ RadioClass  vtable @ 0x007F0508
                      └─ TechnoClass  vtable @ 0x007F4960
                           └─ BuildingClass  vtable @ 0x007E3EBC
```

**Note:** There is NO FootClass in the BuildingClass inheritance chain. Buildings inherit directly from TechnoClass, unlike InfantryClass/UnitClass/AircraftClass which go through FootClass.

### Approximate slot ranges by class:
| Class | Slots | Notes |
|-------|-------|-------|
| IUnknown | 0–2 | QueryInterface, AddRef, Release |
| AbstractClass | 3–22 | Init, GetType, ComputeCRC, etc. |
| ObjectClass | 23–98 | Coords, Draw, Unlimbo, Limbo, ReceiveDamage, etc. |
| MissionClass | 97–127 | GetCurrentMission, Assign_Mission, Queue_Mission, Mission handlers dispatch |
| RadioClass | 157–160 | Transmit_Radio variants |
| TechnoClass | 161–299 | Cloak, weapons, combat, AI, production, building-specific extensions |

---

## 3. Complete Vtable Map — ALL 300 Slots

Legend: **B** = BuildingClass override, **I** = Inherited from parent

### IUnknown (slots 0–2)

| Slot | Offset | Address | Name | Override? |
|------|--------|---------|------|-----------|
| 0 | 0x000 | 0x00410260 | AbstractClass::QueryInterface | I |
| 1 | 0x004 | 0x00410300 | AbstractClass::AddRef | I |
| 2 | 0x008 | 0x00410310 | AbstractClass::Release | I |

### AbstractClass (slots 3–22)

| Slot | Offset | Address | Name | Override? |
|------|--------|---------|------|-----------|
| 3 | 0x00C | 0x00459E80 | Init (stub/return) | **B** |
| 4 | 0x010 | 0x00410450 | AbstractClass::GetOwnerHouse | I |
| 5 | 0x014 | 0x00453E20 | AbstractClass::Save | **B** |
| 6 | 0x018 | 0x00454190 | AbstractClass::Load | **B** |
| 7 | 0x01C | 0x004103E0 | AbstractClass::ComputeCRC | I |
| 8 | 0x020 | 0x00459F20 | AbstractClass::WhatAmI (returns building type ID) | **B** |
| 9 | 0x024 | 0x00442C40 | AbstractClass::SizeOf | **B** |
| 10 | 0x028 | 0x0044E8F0 | BuildingClass::GetType (GetObjectTypeClass) | **B** |
| 11 | 0x02C | 0x00459EC0 | AbstractClass::vt_func_11 (stub) | **B** |
| 12 | 0x030 | 0x00459E70 | AbstractClass::vt_func_12 (stub) | **B** |
| 13 | 0x034 | 0x00454260 | AbstractClass::PointerExpired | **B** |
| 14 | 0x038 | 0x006F9DB0 | GetOwnerHouseID | I |
| 15 | 0x03C | 0x006F9DC0 | GetOwnerHouseType | I |
| 16 | 0x040 | 0x004104B0 | AbstractClass::IsActive | I |
| 17 | 0x044 | 0x005F6690 | ObjectClass::IsDead | I |
| 18 | 0x048 | 0x00447AC0 | BuildingClass::GetCoords | **B** |
| 19 | 0x04C | 0x00447E90 | BuildingClass::GetDestination | **B** |
| 20 | 0x050 | 0x005F6B60 | ObjectClass::vt_func_20 | I |
| 21 | 0x054 | 0x005F6B90 | ObjectClass::vt_func_21 | I |
| 22 | 0x058 | 0x00410540 | AbstractClass::vt_func_22 | I |

### ObjectClass (slots 23–98)

| Slot | Offset | Address | Name | Override? |
|------|--------|---------|------|-----------|
| 23 | 0x05C | 0x0043FB20 | **BuildingClass::Update (AI per-tick)** | **B** |
| 24 | 0x060 | 0x00710410 | ObjectClass::SaveLoad_ReadFromINI | I |
| 25 | 0x064 | 0x006F32D0 | BuildingTypeClass::ReadINI (trampoline) | I |
| 26 | 0x068 | 0x004544A0 | BuildingClass::GetVisualState | **B** |
| 27 | 0x06C | 0x004513D0 | BuildingClass::vt_func_27 (anim related) | **B** |
| 28 | 0x070 | 0x00447540 | BuildingClass::vt_func_28 (GrandOpening related) | **B** |
| 29 | 0x074 | 0x00447210 | BuildingClass::vt_func_29 | **B** |
| 30 | 0x078 | 0x005F4260 | ObjectClass::vt_func_30 | I |
| 31 | 0x07C | 0x005F6C10 | ObjectClass::vt_func_31 | I |
| 32 | 0x080 | 0x00457620 | BuildingClass::GetSuperWeapon | **B** |
| 33 | 0x084 | 0x006F3270 | TechnoClass::GetTechnoType (trampoline) | I |
| 34 | 0x088 | 0x00459EE0 | BuildingClass::vt_func_34 (stub/return 0) | **B** |
| 35 | 0x08C | 0x00708B30 | TechnoClass::vt_func_35 | I |
| 36 | 0x090 | 0x00459ED0 | BuildingClass::GetPixelSelectionBracketDelta (stub) | **B** |
| 37 | 0x094 | 0x00452630 | BuildingClass::CanAcceptUpgrade | **B** |
| 38 | 0x098 | 0x004494C0 | BuildingClass::ClearBibArea | **B** |
| 39 | 0x09C | 0x007010D0 | TechnoClass::vt_func_39 | I |
| 40 | 0x0A0 | 0x0044F5C0 | BuildingClass::ShouldShowDeployButton | **B** |
| 41 | 0x0A4 | 0x004500A0 | BuildingClass::GetTargetCoords | **B** |
| 42 | 0x0A8 | 0x00447B20 | BuildingClass::GetDockCoord | **B** |
| 43 | 0x0AC | 0x00459EF0 | BuildingClass::GetRenderCoords | **B** |
| 44 | 0x0B0 | 0x00453840 | BuildingClass::vt_func_44 (cell occupancy) | **B** |
| 45 | 0x0B4 | 0x0044F640 | BuildingClass::vt_func_45 | **B** |
| 46 | 0x0B8 | 0x00449410 | BuildingClass::vt_func_46 | **B** |
| 47 | 0x0BC | 0x005F6A70 | ObjectClass::ShouldBeOnBridge | I |
| 48 | 0x0C0 | 0x00426410 | vt_func_48 | I |
| 49 | 0x0C4 | 0x0041C010 | vt_func_49 (stub) | I |
| 50 | 0x0C8 | 0x0041C020 | vt_func_50 (stub) | I |
| 51 | 0x0CC | 0x0041BE60 | vt_func_51 | I |
| 52 | 0x0D0 | 0x0041BE70 | vt_func_52 | I |
| 53 | 0x0D4 | 0x00445880 | **BuildingClass::OnDestroyed** | **B** |
| 54 | 0x0D8 | 0x00440580 | **BuildingClass::Unlimbo** | **B** |
| 55 | 0x0DC | 0x0044EBF0 | BuildingClass::Limbo/Destroy | **B** |
| 56 | 0x0E0 | 0x00702D40 | TechnoClass::RecordKill | I |
| 57 | 0x0E4 | 0x00703230 | TechnoClass::vt_func_57 | I |
| 58 | 0x0E8 | 0x005F5940 | ObjectClass::Unlimbo (base) | I |
| 59 | 0x0EC | 0x005F4160 | ObjectClass::DropIn | I |
| 60 | 0x0F0 | 0x00453D60 | BuildingClass::Mark_Put | **B** |
| 61 | 0x0F4 | 0x00453DC0 | BuildingClass::Mark_Remove | **B** |
| 62 | 0x0F8 | 0x005F65F0 | ObjectClass::Limbo | I |
| 63 | 0x0FC | 0x00703850 | TechnoClass::vt_func_63 | I |
| 64 | 0x100 | 0x00443C60 | **BuildingClass::ExitObject (exit transport)** | **B** |
| 65 | 0x104 | 0x0043CEA0 | BuildingClass::DrawSomething | **B** |
| 66 | 0x108 | 0x005F5B90 | ObjectClass::vt_func_66 | I |
| 67 | 0x10C | 0x006F60D0 | TechnoClass::vt_func_67 | I |
| 68 | 0x110 | 0x006F5190 | TechnoClass::vt_func_68 | I |
| 69 | 0x114 | 0x0043D290 | **BuildingClass::DrawBody** | **B** |
| 70 | 0x118 | 0x0043D030 | BuildingClass::DrawSomething_2 | **B** |
| 71 | 0x11C | 0x006F4A40 | TechnoClass::vt_func_71 | I |
| 72 | 0x120 | 0x0070ADC0 | TechnoClass::vt_func_72 | I |
| 73 | 0x124 | 0x0043F180 | BuildingClass::SetMissionAndAnims | **B** |
| 74 | 0x128 | 0x005F4730 | ObjectClass::vt_func_74 | I |
| 75 | 0x12C | 0x00455C20 | BuildingClass::vt_func_75 | **B** |
| 76 | 0x130 | 0x00456750 | BuildingClass::vt_func_76 (sensor related) | **B** |
| 77 | 0x134 | 0x005F4D10 | ObjectClass::vt_func_77 | I |
| 78 | 0x138 | 0x005F6C30 | ObjectClass::vt_func_78 | I |
| 79 | 0x13C | 0x00459C00 | BuildingClass::vt_func_79 | **B** |
| 80 | 0x140 | 0x004436F0 | BuildingClass::SetRallyPoint | **B** |
| 81 | 0x144 | 0x00443410 | BuildingClass::vt_func_81 | **B** |
| 82 | 0x148 | 0x00456E00 | BuildingClass::vt_func_82 | **B** |
| 83 | 0x14C | 0x006FBFA0 | TechnoClass::vt_func_83 | I |
| 84 | 0x150 | 0x005F44A0 | ObjectClass::vt_func_84 | I |
| 85 | 0x154 | 0x00457C90 | **BuildingClass::IronCurtain** | **B** |
| 86 | 0x158 | 0x0070E340 | BuildingClass::SetCoords (inherited impl) | I |
| 87 | 0x15C | 0x0070E300 | TechnoClass::vt_func_87 | I |
| 88 | 0x160 | 0x0041BF40 | vt_func_88 | I |
| 89 | 0x164 | 0x006F7970 | TechnoClass::vt_func_89 | I |
| 90 | 0x168 | 0x007012C0 | TechnoClass::vt_func_90 | I |
| 91 | 0x16C | 0x00442230 | **BuildingClass::ReceiveDamage** | **B** |
| 92 | 0x170 | 0x00710460 | TechnoClass::vt_func_92 | I |
| 93 | 0x174 | 0x005F43A0 | ObjectClass::vt_func_93 | I |
| 94 | 0x178 | 0x005F43B0 | ObjectClass::vt_func_94 | I |
| 95 | 0x17C | 0x005F43C0 | ObjectClass::vt_func_95 | I |
| 96 | 0x180 | 0x00707DD0 | TechnoClass::vt_func_96 | I |
| 97 | 0x184 | 0x005B3040 | MissionClass::GetCurrentMission | I |
| 98 | 0x188 | 0x0041BE90 | vt_func_98 | I |

### MissionClass / RadioClass continuation (slots 99–160)

| Slot | Offset | Address | Name | Override? |
|------|--------|---------|------|-----------|
| 99 | 0x18C | 0x006F5090 | TechnoClass::vt_func_99 | I |
| 100 | 0x190 | 0x005F5C20 | ObjectClass::vt_func_100 | I |
| 101 | 0x194 | 0x0043C2D0 | **BuildingClass::Receive_Radio** | **B** |
| 102 | 0x198 | 0x0044D5D0 | BuildingClass::vt_func_102 | **B** |
| 103 | 0x19C | 0x00446FF0 | BuildingClass::vt_func_103 | **B** |
| 104 | 0x1A0 | 0x00447110 | BuildingClass::TogglePowerOrGate | **B** |
| 105 | 0x1A4 | 0x005F6B50 | ObjectClass::vt_func_105 | I |
| 106 | 0x1A8 | 0x005F4410 | ObjectClass::vt_func_106 | I |
| 107 | 0x1AC | 0x00449440 | BuildingClass::vt_func_107 | **B** |
| 108 | 0x1B0 | 0x004264D0 | vt_func_108 | I |
| 109–116 | 0x1B4–0x1D0 | various | ObjectClass stubs | I |
| 117 | 0x1D4 | 0x0070C5B0 | TechnoClass::IsWarpingOut | I |
| 118 | 0x1D8 | 0x0070C5C0 | TechnoClass::IsBeingWarped | I |
| 119 | 0x1DC | 0x0070C5D0 | TechnoClass::vt_func_119 | I |
| 120 | 0x1E0 | 0x0070C5F0 | TechnoClass::IsNotWarping | I |
| 121 | 0x1E4 | 0x00705D70 | TechnoClass::vt_func_121 | I |
| 122 | 0x1E8 | 0x005B35E0 | MissionClass::Queue_Mission | I |
| 123 | 0x1EC | 0x005B3570 | MissionClass::Commence | I |
| 124 | 0x1F0 | 0x005B2FD0 | MissionClass::Assign_Mission | I |
| 125 | 0x1F4 | 0x007013A0 | TechnoClass::vt_func_125 | I |
| 126 | 0x1F8 | 0x007013E0 | TechnoClass::vt_func_126 | I |
| 127 | 0x1FC | 0x005B3A10 | MissionClass::Is_Mission_Suspended | I |

### Mission Handler Slots (128–156) — SEE SECTION 5 FOR DETAILS

| Slot | Offset | Address | Name | Override? |
|------|--------|---------|------|-----------|
| 128 | 0x200 | 0x00454250 | BuildingClass::PointerExpired (repurposed) | **B** |
| 129 | 0x204 | 0x005B2E10 | MissionClass::Mission_Default (Sleep dispatch) | I |
| 130 | 0x208 | 0x005B2E20 | Mission_Harmless/Open | I |
| 131 | 0x20C | 0x005B2E30 | Mission_Enter | I |
| 132 | 0x210 | 0x0044ACF0 | **BuildingClass::Mission_Attack** | **B** |
| 133 | 0x214 | 0x0044B760 | **BuildingClass::Mission_Guard** | **B** |
| 134 | 0x218 | 0x005B2E60 | Mission_AreaGuard (default) | I |
| 135 | 0x21C | 0x004496B0 | **BuildingClass::Mission_Retreat** | **B** |
| 136 | 0x220 | 0x00449A40 | **BuildingClass::Mission_Stop** | **B** |
| 137 | 0x224 | 0x0044B770 | **BuildingClass::Mission_Return/Repair** | **B** |
| 138 | 0x228 | 0x005B2EA0 | Mission_Capture (default) | I |
| 139 | 0x22C | 0x005B2EB0 | Mission_Move (default) | I |
| 140 | 0x230 | 0x005B2EC0 | Mission_QMove (default) | I |
| 141 | 0x234 | 0x005B2ED0 | Mission_Ambush (default) | I |
| 142 | 0x238 | 0x005B2EE0 | Mission_Hunt (default) | I |
| 143 | 0x23C | 0x0044D880 | **BuildingClass::Mission_Rescue** | **B** |
| 144 | 0x240 | 0x005B2F00 | Mission_Harvest (default) | I |
| 145 | 0x244 | 0x00449A50 | **BuildingClass::Mission_Construction** | **B** |
| 146 | 0x248 | 0x00449C30 | **BuildingClass::Mission_Selling** | **B** |
| 147 | 0x24C | 0x0044B780 | **BuildingClass::Mission_RepairAndProduce** | **B** |
| 148 | 0x250 | 0x0044C980 | **BuildingClass::Mission_Missile** | **B** |
| 149 | 0x254 | 0x0044E440 | **BuildingClass::Mission_Unload** | **B** |
| 150–156 | 0x258–0x270 | 0x005B2Fxx | Default stubs (Sabotage, Timed, ParaWait, etc.) | I |

### RadioClass (slots 157–160)

| Slot | Offset | Address | Name | Override? |
|------|--------|---------|------|-----------|
| 157 | 0x274 | 0x0065ACB0 | RadioClass::Transmit_Radio_ToFirst | I |
| 158 | 0x278 | 0x0065AAA0 | RadioClass::Transmit_Radio | I |
| 159 | 0x27C | 0x0065A970 | RadioClass::Transmit_Radio_Impl | I |
| 160 | 0x280 | 0x0065ACE0 | RadioClass::Broadcast_Radio_ToAll | I |

### TechnoClass (slots 161–299)

| Slot | Offset | Address | Name | Override? |
|------|--------|---------|------|-----------|
| 161 | 0x284 | 0x00455DA0 | BuildingClass::vt_func_161 | **B** |
| 162 | 0x288 | 0x0070C5A0 | TechnoClass::HasStealthAbility | I |
| 163 | 0x28C | 0x006F3280 | TechnoClass::vt_func_163 | I |
| 164 | 0x290 | 0x00459D80 | vt_func_164 | I |
| 165 | 0x294 | 0x0070BE80 | TechnoClass::vt_func_165 | I |
| 166 | 0x298 | 0x006F9E10 | TechnoClass::vt_func_166 | I |
| 167 | 0x29C | 0x0041BEF0 | vt_func_167 (stub) | I |
| 168 | 0x2A0 | 0x00457770 | **BuildingClass::CanCloak** | **B** |
| 169 | 0x2A4 | 0x004578C0 | **BuildingClass::ShouldUncloak** | **B** |
| 170 | 0x2A8 | 0x00445E50 | BuildingClass::vt_func_170 | **B** |
| 171 | 0x2AC | 0x00458DB0 | BuildingClass::vt_func_171 | **B** |
| 172 | 0x2B0 | 0x0070C620 | TechnoClass::IsNotAtDestination | I |
| 173 | 0x2B4 | 0x00708BC0 | TechnoClass::vt_func_173 | I |
| 174 | 0x2B8 | 0x0044D700 | BuildingClass::vt_func_174 | **B** |
| 175 | 0x2BC | 0x0070ADA0 | TechnoClass::vt_func_175 | I |
| 176 | 0x2C0 | 0x00708B40 | TechnoClass::vt_func_176 | I |
| 177 | 0x2C4 | 0x00459D90 | vt_func_177 | I |
| 178 | 0x2C8 | 0x0043E940 | BuildingClass::vt_func_178 (anim/frame) | **B** |
| 179 | 0x2CC | 0x00707F60 | TechnoClass::vt_func_179 | I |
| 180 | 0x2D0 | 0x00451330 | BuildingClass::vt_func_180 (anim setup) | **B** |
| 181 | 0x2D4 | 0x00459870 | BuildingClass::vt_func_181 (stub) | **B** |
| 182 | 0x2D8 | 0x00459880 | BuildingClass::CanCloak_vt2a0 | **B** |
| 183 | 0x2DC | 0x00459890 | BuildingClass::ShouldUncloak_2 | **B** |
| 184 | 0x2E0 | 0x004576F0 | BuildingClass::GetSuperWeaponIndex | **B** |
| 185 | 0x2E4 | 0x006F3330 | TechnoClass::vt_func_185 | I |
| 186 | 0x2E8 | 0x006F3820 | TechnoClass::vt_func_186 | I |
| 187 | 0x2EC | 0x0043E900 | BuildingClass::vt_func_187 | **B** |
| 188–191 | 0x2F0–0x2FC | various | stubs/inherited | I |
| 192 | 0x300 | 0x00453A70 | BuildingClass::vt_func_192 | **B** |
| 193 | 0x304 | 0x00708C10 | TechnoClass::vt_func_193 | I |
| 194 | 0x308 | 0x0044D7D0 | BuildingClass::vt_func_194 | **B** |
| 195 | 0x30C | 0x0044EB10 | BuildingClass::vt_func_195 | **B** |
| 196–199 | 0x310–0x31C | various | TechnoClass inherited | I |
| 200 | 0x320 | 0x00459DD0 | vt_func_200 | I |
| 201 | 0x324 | 0x00457020 | BuildingClass::vt_func_201 | **B** |
| 202–203 | 0x328–0x32C | TechnoClass | inherited | I |
| 204 | 0x330 | 0x0041BF30 | vt_func_204 (stub) | I |
| 205 | 0x334 | 0x00459DE0 | vt_func_205 | I |
| 206 | 0x338 | 0x0070F8F0 | TechnoClass::vt_func_206 | I |
| 207–211 | 0x33C–0x34C | various | stubs/inherited | I |
| 212 | 0x350 | 0x004555D0 | **BuildingClass::CanSellOrUndeploy** | **B** |
| 213–218 | 0x354–0x368 | various | TechnoClass combat/cloaking | I |
| 219 | 0x36C | 0x00459C20 | BuildingClass::vt_func_219 | **B** |
| 220–237 | 0x370–0x3B4 | various | TechnoClass inherited | I |
| 238 | 0x3B8 | 0x0044D760 | BuildingClass::vt_func_238 | **B** |
| 239 | 0x3BC | 0x006FC090 | TechnoClass::vt_func_239 | I |
| 240 | 0x3C0 | 0x00447F10 | **BuildingClass::GetFireError** | **B** |
| 241 | 0x3C4 | 0x00445F00 | BuildingClass::vt_func_241 | **B** |
| 242 | 0x3C8 | 0x00443B90 | **BuildingClass::ToggleGate** | **B** |
| 243 | 0x3CC | 0x006FDD50 | TechnoClass::vt_func_243 | I |
| 244 | 0x3D0 | 0x0070F850 | TechnoClass::vt_func_244 | I |
| 245 | 0x3D4 | 0x00448260 | **BuildingClass::ChangeOwner** | **B** |
| 246 | 0x3D8 | 0x0070B280 | TechnoClass::vt_func_246 | I |
| 247 | 0x3DC | 0x00459E50 | vt_func_247 (stub) | I |
| 248–253 | 0x3E0–0x3F4 | various | TechnoClass inherited | I |
| 254 | 0x3F8 | 0x004526F0 | **BuildingClass::GetWeapon** | **B** |
| 255 | 0x3FC | 0x004527D0 | BuildingClass::vt_func_255 | **B** |
| 256 | 0x400 | 0x00458DD0 | **BuildingClass::IsOccupied** | **B** |
| 257 | 0x404 | 0x00458E00 | **BuildingClass::GetHalfFoundationSize** | **B** |
| 258 | 0x408 | 0x004581F0 | **BuildingClass::GetOccupantCount** | **B** |
| 259 | 0x40C | 0x00701410 | TechnoClass::EngineerRepair | I |
| 260 | 0x410 | 0x00454DB0 | **BuildingClass::UpdateGapGenerator_Tick** | **B** |
| 261–269 | 0x414–0x434 | various | TechnoClass inherited | I |
| 270 | 0x438 | 0x00459E60 | vt_func_270 (stub) | I |
| 271–280 | 0x43C–0x460 | various | TechnoClass inherited | I |
| 281 | 0x464 | 0x00456F80 | **BuildingClass::AdjustZHeight** | **B** |
| 282 | 0x468 | 0x00459900 | BuildingClass::vt_func_282 | **B** |
| 283 | 0x46C | 0x0070E280 | TechnoClass::vt_func_283 | I |
| 284 | 0x470 | 0x0041C030 | vt_func_284 (stub) | I |
| 285–287 | 0x474–0x47C | various | inherited | I |
| 288 | 0x480 | 0x00455D50 | BuildingClass::vt_func_288 | **B** |
| 289 | 0x484 | 0x0044D6A0 | BuildingClass::vt_func_289 | **B** |
| 290–291 | 0x488–0x48C | TechnoClass | inherited | I |
| 292 | 0x490 | 0x00458A80 | BuildingClass::vt_func_292 | **B** |
| 293 | 0x494 | 0x00456580 | **BuildingClass::RegisterOnRadar** | **B** |
| 294 | 0x498 | 0x004565E0 | BuildingClass::vt_func_294 | **B** |
| 295 | 0x49C | 0x00456640 | BuildingClass::vt_func_295 | **B** |
| 296–299 | 0x4A0–0x4AC | various | TechnoClass inherited | I |

---

## 4. Key Lifecycle Methods

### Constructor
- **Address:** `0x0043B740` (main constructor with BuildingTypeClass param)
- **Thin wrapper:** `0x0043B680` (delegating constructor)
- Sets vtable pointers to `vtable_BuildingClass` and 3 secondary vtables
- Calls `TechnoClass::Constructor` first
- Initializes ~200 fields including timers, anim slots, upgrade slots
- Adds self to `g_BuildingClass_Array` and global tracking arrays
- Sets radio contact count from `BuildingTypeClass+0x1780` (NumberOfDocks)

### Destructor
- **Address:** `0x0043BCF0` (corrected 2026-05-29: was "mislabeled as `BuildingClass__Constructor` in Ghidra"; Ghidra now shows correct label `BuildingClass__Destructor` — STALE note, label was fixed in Ghidra since original report; verified via get_function_by_address 0x0043BCF0)
- Sets vtable back to BuildingClass (standard C++ destructor pattern)
- Releases sound events, building light sources, factory references
- Calls `BuildingClass::OnDestroyed` for cleanup
- Removes from `g_BuildingClass_Array` and global tracking
- Clears all anim slots, damage fire anims
- Updates gap generator if applicable
- Manages superweapon ownership tracking
- Calls `MissionClass::Constructor` (actually MissionClass destructor)

### Unlimbo (Place on Map)
- **Address:** `0x00440580` — vtable slot 54 (offset 0x0D8)
- **Huge function** (~4,300 bytes) — most complex lifecycle method
- Steps:
  1. Checks if placement cell is valid; handles wall/upgrade placement specially
  2. For walls: auto-extends in 4 directions, recalculates connections
  3. For upgrades: validates same owner, attaches to parent building
  4. Calls `TechnoClass::Unlimbo` 
  5. Sets up fog of war reveal if gap generator
  6. Creates building light source (`BuildingLightClass`) if HasSpotlight
  7. Creates particle system if building type defines one
  8. Registers in HouseClass tracking arrays:
     - Radar buildings, sensor array buildings, laser fence buildings
     - Gap generators, factory buildings, docking buildings
     - SpySat buildings, power drain buildings
  9. Manages cell pathability (increments building-adjacent cell occupancy counters)
  10. Recalculates HouseClass base center
  11. Updates radar if building has RadarJammer

### Limbo / Destroy
- **Address:** `0x0044EBF0` — vtable slot 55 (offset 0x0DC)
- Handles removal from map (cleanup reverse of Unlimbo)

### Update (AI Per-Tick)
- **Address:** `0x0043FB20` — vtable slot 23 (offset 0x05C)
- Called every tick for each living building
- Steps:
  1. Checks if building is powered/online; updates looping sounds
  2. Updates damage state fire anims (creates/removes based on health ratio)
  3. Updates occupant frame counter for IdleRate
  4. Checks docked aircraft status
  5. Handles cash-generating buildings (CashDelay/CashAmount per tick)
  6. Updates building animation state
  7. Calls `TechnoClass::AI_Update`
  8. Manages IFV-style weapon cycling for occupied buildings
  9. Updates building-specific delayed effects (bridge destruction countdown)
  10. Processes delayed fire

### OnDestroyed
- **Address:** `0x00445880` — vtable slot 53 (offset 0x0D4)
- Handles building death effects, survivor spawning, EVA notifications

### Draw / DrawBody
- **Draw entry:** `0x004E0240` — vtable offset unknown (indirect call to DrawBody)
- **DrawBody:** `0x0043D290` — vtable slot 69 (offset 0x114)
- Renders the building sprite, turret, upgrades, fire anims, power-down overlay

### ReceiveDamage
- **Address:** `0x00442230` — vtable slot 91 (offset 0x16C)
- Building-specific damage handling, bridge logic, OnDestroyed dispatch

### ExitObject (Unit Exit)
- **Address:** `0x00443C60` — vtable slot 64 (offset 0x100)
- Handles units exiting the building (production complete, garrison eject, etc.)

---

## 5. Mission Handlers — Complete BuildingClass Map

The mission dispatch is NOT sequential — enum values map to vtable offsets via a switch table in `MissionClass::Mission_Dispatch` at `0x005B3060`.

### Mission Enum → Vtable Offset Mapping (from binary)

| Enum | Name | Vtable Offset | Vtable Slot | BuildingClass Address | Override? |
|------|------|---------------|-------------|----------------------|-----------|
| 0 | Sleep | 0x204 | 129 | 0x005B2E10 (default) | No |
| 1 | Attack | 0x210 | 132 | **0x0044ACF0** | **YES** |
| 2 | Move | 0x22C | 139 | 0x005B2EB0 (default) | No |
| 4 | QMove | 0x230 | 140 | 0x005B2EC0 (default) | No |
| 5/6 | Retreat | 0x21C | 135 | **0x004496B0** | **YES** |
| 7 | Harvest | 0x240 | 144 | 0x005B2F00 (default) | No |
| 8 | Guard | 0x214 | 133 | **0x0044B760** | **YES** |
| 9 | AreaGuard | 0x218 | 134 | 0x005B2E60 (default) | No |
| 10 | Return | 0x224 | 137 | **0x0044B770** | **YES** |
| 11 | Stop | 0x220 | 136 | **0x00449A40** | **YES** |
| 14 | Enter | 0x20C | 131 | 0x005B2E30 (default) | No |
| 15 | Capture | 0x228 | 138 | 0x005B2EA0 (default) | No |
| 16 | Eaten | 0x23C | 143 | **0x0044D880** | **YES** |
| 17 | Harmless | 0x214 | 133 | **0x0044B760** (reuses Guard) | **YES** |
| 18 | Construction | 0x244 | 145 | **0x00449A50** | **YES** |
| 19 | Selling | 0x248 | 146 | **0x00449C30** | **YES** |
| 20 | Repair | 0x24C | 147 | **0x0044B780** | **YES** |
| 21 | Rescue | 0x258 | 150 | 0x005B2F60 (default) | No |
| 22 | Missile | 0x250 | 148 | **0x0044C980** | **YES** |
| 23 | Open/Harmless | 0x208 | 130 | 0x005B2E20 (default) | No |
| 24 | Unload | 0x254 | 149 | **0x0044E440** | **YES** |

### BuildingClass Mission Handler Details

**Mission_Attack** (`0x0044ACF0`, ~1,174 bytes)
- Handles building weapon fire logic
- Checks target validity, weapon availability, fire error conditions
- Manages turret rotation to target
- Returns delay until next tick

**Mission_Guard** (`0x0044B760`, ~26 bytes)  
- Very short — likely just resets to idle or delegates
- Used for both Guard (enum 8) and Harmless (enum 17) missions

**Mission_Construction** (`0x00449A50`, ~434 bytes)
- Handles the building construction animation sequence
- Monitors construction progress via factory

**Mission_Selling** (`0x00449C30`, ~3,989 bytes)
- Massive function — handles the entire sell sequence
- Manages sell animation, credit return, MCV undeploy
- Spawns survivors/units as appropriate

**Mission_RepairAndProduce** (`0x0044B780`, ~4,604 bytes)
- The primary "operational" mission for most buildings
- Handles unit/aircraft production
- Manages repair operations
- Handles dock operations and unit exit

**Mission_Missile** (`0x0044C980`, ~3,104 bytes)
- Handles nuclear missile / superweapon silo launch sequence
- Multi-stage: open doors → raise missile → launch

**Mission_Unload** (`0x0044E440`, ~unlabeled, likely small)
- Handles unloading passengers/cargo

**Mission_Retreat** (`0x004496B0`, ~902 bytes)
- Building-specific retreat handling

**Mission_Stop** (`0x00449A40`, ~8 bytes)
- Very small stub — just returns

**Mission_Return** (`0x0044B770`, ~16 bytes)
- Small — likely just transitions to another mission

**Mission_Rescue** (`0x0044D880`, ~unlabeled)
- Mission handler for "Eaten" enum (16) — building-specific

---

## 6. Total Override Count

BuildingClass overrides **~95 out of 300** vtable slots from TechnoClass. The heaviest override areas are:
- Object lifecycle (Unlimbo, Destroy, ReceiveDamage, ExitObject)
- Coordinates and positioning (GetCoords, GetRenderCoords, GetTargetCoords, GetDockCoord)
- Drawing (DrawBody, Draw helper)
- Mission handlers (12 of 29 mission slots overridden)
- Building-specific: GetWeapon, IronCurtain, ChangeOwner, CanCloak, ToggleGate, RegisterOnRadar, AdjustZHeight, GapGenerator, GetFireError, CanSellOrUndeploy, occupancy/foundation queries

---

## 7. BuildingClass Instance Size

From constructor at `0x0043B740`: param_1 type is `int *` (pointer), and the highest indexed field is `param_1[0x1C6]` = byte offset 0x718. Combined with trailing byte fields, estimated struct size is approximately **0x720–0x730 bytes** (~1,824–1,840 bytes).

The global array is at `g_BuildingClass_Array` (`0x00A8EB44`), count at `0x00A8EB50`.
