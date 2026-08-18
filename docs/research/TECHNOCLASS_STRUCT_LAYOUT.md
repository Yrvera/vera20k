# TechnoClass Struct Layout (gamemd.exe)

Research date: 2026-03-21
Source: Ghidra MCP decompilation of gamemd.exe
Method: Decompiled 10 key TechnoClass functions and extracted all field offsets from `this` pointer accesses.

## Inheritance

```
AbstractClass -> ObjectClass (ends ~0xAC) -> TechnoClass -> BuildingClass / UnitClass / InfantryClass / AircraftClass
```

TechnoClass inherits from ObjectClass. ObjectClass fields occupy bytes 0x00-0xAB.
TechnoClass's own fields begin at byte offset 0xF0 (index 0x3C).

**Note on Ghidra output**: `param_1` is typed as `undefined4*` (4-byte pointer), so `param_1[N]` = byte offset `N * 4`. Direct byte accesses use `(int)param_1 + 0xNNN`.

## VTable Pointers (bytes 0x00-0x0F)

| Byte Offset | Index  | Type      | Purpose                          | Function(s)           | Confidence |
|-------------|--------|-----------|----------------------------------|-----------------------|------------|
| 0x000       | [0]    | vtable*   | Primary vtable (TechnoClass)     | Constructor           | HIGH       |
| 0x004       | [1]    | vtable*   | Secondary vtable                 | Constructor           | HIGH       |
| 0x008       | [2]    | vtable*   | Tertiary vtable                  | Constructor           | HIGH       |
| 0x00C       | [3]    | vtable*   | Quaternary vtable                | Constructor           | HIGH       |

## ObjectClass Fields (0x00-0xAB) — Inherited, not detailed here

Key inherited fields referenced:
- `[5]` (0x14): Flags byte — bit 0 = IsOnMap, bit 1 = exists/valid, bit 2 = IsBuilding
- `[0x19D]` (0x674): Referenced in GetFLH — but this is TechnoClass range (see below)

## TechnoClass Own Fields (0xF0+)

### Sorted by byte offset

| Byte Offset | Index   | Type             | Purpose / Usage                                                          | Function(s)                                      | Confidence |
|-------------|---------|------------------|--------------------------------------------------------------------------|--------------------------------------------------|------------|
| 0x0F0       | [0x3C]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x0F4       | [0x3D]  | byte (at +0)     | Unknown bool — initialized to 0                                         | Constructor                                       | MED        |
| 0x0F8       | [0x3E]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x0FC       | [0x3F]  | byte (at +0)     | Unknown bool — initialized to 0                                         | Constructor                                       | MED        |
| 0x100       | [0x40]  | int              | Timer/frame counter — set to g_CurrentFrameCounter                      | Constructor                                       | HIGH       |
| 0x108       | [0x42]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x10C       | [0x43]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x110       | [0x44]  | int              | Unknown — initialized to 1                                              | Constructor                                       | MED        |
| 0x114       | [0x45]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x118       | [0x46]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x11C       | [0x47]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x120       | [0x48]  | int              | **LastFireFrame** — set to g_CurrentFrameCounter after firing            | Fire_At (end)                                     | HIGH       |
| 0x124       | [0x49]  | int              | Unknown — initialized to -1 (0xFFFFFFFF)                                | Constructor                                       | MED        |
| 0x128       | [0x4A]  | int              | Unknown — initialized to -1 (0xFFFFFFFF)                                | Constructor                                       | MED        |
| 0x12C       | [0x4B]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x130       | [0x4C]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x134       | [0x4D]  | byte (at +0)     | Unknown bool — initialized to 0                                         | Constructor                                       | MED        |
| 0x138       | [0x4E]  | int              | **ForcedWeaponIndex** — used in SelectWeaponAgainst; checked != -1       | SelectWeaponAgainst, Constructor                  | HIGH       |
| 0x13C       | [0x4F]  | int              | Unknown — initialized to -1 (0xFFFFFFFF)                                | Constructor                                       | MED        |
| 0x140       | [0x50]  | int              | **CurrentWeaponIndex** — used in weapon selection logic (iVar4 * 2)      | SelectWeaponAgainst                               | HIGH       |
| 0x144       | [0x51]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x148       | [0x52]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x14C       | [0x53]  | param             | **TechnoTypeClass ptr** — set to constructor param (type pointer)        | Constructor                                       | HIGH       |
| 0x158       | [0x56]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x15C       | [0x57]  | float / double_hi | Part of double — initialized to 0x3FF00000 (1.0 as high dword)          | Constructor                                       | MED        |
| 0x160       | [0x58]  | float / double_lo | Part of double — initialized to 0                                       | Constructor                                       | MED        |
| 0x164       | [0x59]  | float / double_hi | Part of double — initialized to 0x3FF00000 (1.0)                        | Constructor                                       | MED        |
| 0x168       | [0x5A]  | int              | Timer — set to g_CurrentFrameCounter                                    | Constructor                                       | MED        |
| 0x170       | [0x5C]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x174       | [0x5D]  | int              | Timer — set to g_CurrentFrameCounter                                    | Constructor                                       | MED        |
| 0x17C       | [0x5F]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x180       | [0x60]  | int              | Timer — set to g_CurrentFrameCounter                                    | Constructor                                       | MED        |
| 0x188       | [0x62]  | int              | Unknown — initialized to 0x2D (45)                                      | Constructor                                       | MED        |
| 0x18C       | [0x63]  | int              | Timer — set to g_CurrentFrameCounter (index 99/0x63)                    | Constructor                                       | MED        |
| 0x194       | [0x65]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x198       | [0x66]  | int              | Timer — set to g_CurrentFrameCounter                                    | Constructor                                       | MED        |
| 0x1A0       | [0x68]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x1A4       | [0x69]  | int              | Unknown — initialized to 10                                             | Constructor                                       | MED        |
| 0x1A8       | [0x6A]  | int              | Timer — set to g_CurrentFrameCounter                                    | Constructor                                       | MED        |
| 0x1B0       | [0x6C]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x1B4       | [0x6D]  | int              | Timer — set to g_CurrentFrameCounter                                    | Constructor                                       | MED        |
| 0x1BC       | [0x6F]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x1C4       | [0x71]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x1C8       | [0x72]  | byte (at +0)     | Unknown bool — initialized to 0                                         | Constructor                                       | MED        |
| 0x1CC       | [0x73]  | int              | **IronCurtainTimer** or similar — checked == 0 in weapon selection       | SelectWeaponAgainst, Constructor                  | MED        |
| 0x1D0       | [0x74]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x1D4       | [0x75]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x1D8       | [0x76]  | byte (at +0)     | Unknown bool — initialized to 0                                         | Constructor                                       | MED        |
| 0x1DC       | [0x77]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x1E0       | [0x78]  | int              | Timer — set to g_CurrentFrameCounter                                    | Constructor                                       | MED        |
| 0x1E8       | [0x7A]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x1EC       | [0x7B]  | int              | Timer — set to g_CurrentFrameCounter                                    | Constructor                                       | MED        |
| 0x1F4       | [0x7D]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x1F8       | [0x7E]  | byte (at +0)     | Unknown bool — initialized to 0                                         | Constructor                                       | MED        |
| 0x1FC       | [0x7F]  | int              | Timer — set to g_CurrentFrameCounter                                    | Constructor                                       | MED        |
| 0x204       | [0x81]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x208       | [0x82]  | int              | Unknown — initialized to 0 (NOTE: byte offset 0x82 accesses in Fire_At refer to ObjectClass byte, not this field) | Constructor                  | MED        |
| 0x20C       | [0x83]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x210       | [0x84]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x214       | [0x85]  | int              | Unknown — initialized to -1 (0xFFFFFFFF)                                | Constructor                                       | MED        |
| 0x218       | [0x86]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x21C       | [0x87]  | ptr (HouseClass*)| **Owner** — set to constructor param; compared to g_PlayerPtr            | Constructor, Uncloak, PerformDeploy               | HIGH       |
| 0x220       | [0x88]  | int/enum         | **CloakState** — compared to 2 in Uncloak (2 = cloaked)                 | Uncloak, Constructor                              | HIGH       |
| 0x224       | [0x89]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x228       | [0x8A]  | byte (at +0)     | Unknown bool — initialized to 0                                         | Constructor                                       | MED        |
| 0x22C       | [0x8B]  | int              | Timer — set to g_CurrentFrameCounter                                    | Constructor                                       | MED        |
| 0x234       | [0x8D]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x238       | [0x8E]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x23C       | [0x8F]  | int              | Unknown — initialized to 1                                              | Constructor                                       | MED        |
| 0x240       | [0x90]  | int              | Timer — set to g_CurrentFrameCounter                                    | Constructor                                       | MED        |
| 0x248       | [0x92]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x24C       | [0x93]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x250       | [0x94]  | byte (at +0)     | Unknown bool — initialized to 0                                         | Constructor                                       | MED        |
| 0x254       | [0x95]  | int              | **ChronoSourceCoordX** — source coord before warp; init to g_NullCoord_Chrono_X (corrected 2026-05-29: was ChronoDestX; binary shows ChronoSphere__WarpUnitsAtCell writes dest to 0x288 not here; constructor plate comment labels this ChronoSourceCoord — decompile_function 0x006f2b40 + ChronoSphere__WarpUnitsAtCell 0x0065ec30 — RTTI_LABEL_DRIFT) | Constructor                                       | MED        |
| 0x258       | [0x96]  | int              | **ChronoSourceCoordY** — source coord before warp; init to g_NullCoord_Chrono_Y (corrected 2026-05-29: was ChronoDestY — RTTI_LABEL_DRIFT) | Constructor                                       | MED        |
| 0x25C       | [0x97]  | int              | **ChronoSourceCoordZ** — source coord before warp; init to g_NullCoord_Chrono_Z (corrected 2026-05-29: was ChronoDestZ — RTTI_LABEL_DRIFT) | Constructor                                       | MED        |
| 0x260       | [0x98]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x264       | [0x99]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x268       | [0x9A]  | byte (at +0)     | Unknown bool — initialized to 0                                         | Constructor                                       | MED        |
| 0x269       |         | byte             | Unknown bool — direct byte access, initialized to 0                     | Constructor                                       | MED        |
| 0x26C       | [0x9B]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x270       | [0x9C]  | byte (at +0)     | **WarpingOut** — set to 1 by teleport state machine when unit is warping out (corrected 2026-05-29: was "Unknown bool"; TeleportLocomotionClass__StateMachineTick state-0 writes 1 here — decompile_function 0x007192f0 — STALE) | Constructor, TeleportLocomotionClass__StateMachineTick | HIGH       |
| 0x271       |         | byte             | **BeingWarped** — set to 1 by InitiateWarp and ChronoSphere when this unit is being teleported (corrected 2026-05-29: was "Unknown bool"; TeleportLocomotionClass__InitiateWarp writes 1 here — decompile_function 0x00719400 — STALE) | Constructor, TeleportLocomotionClass__InitiateWarp, ChronoSphere__WarpUnitsAtCell | HIGH       |
| 0x272       |         | byte             | Unknown bool — initialized to 0                                         | Constructor                                       | MED        |
| 0x274       | [0x9D]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x278       | [0x9E]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x27C       | [0x9F]  | byte (at +0)     | **ChronoInTransit** — set by teleport code; checked in StateMachineTick (corrected 2026-05-29: was "Unknown bool"; StateMachineTick state-2 writes to +0x27C — decompile_function 0x007192f0 — STALE) | Constructor, TeleportLocomotionClass__StateMachineTick | HIGH       |
| 0x280       | [0xA0]  | int              | **PendingWarpPhase** — set to 3 by ChronoSphere to trigger teleport state 3; set to 0 by InitiateWarp (corrected 2026-05-29: was "Unknown int"; ChronoSphere__WarpUnitsAtCell writes piVar6[0xa0]=3 — decompile_function 0x0065ec30 — STALE) | Constructor, ChronoSphere__WarpUnitsAtCell, TeleportLocomotionClass__StateMachineTick | HIGH       |
| 0x284       | [0xA1]  | int              | **ChronoLockDuration** — set to Rules->ChronoReinfDelay by ChronoSphere; read by teleport state machine (corrected 2026-05-29: was "Unknown int"; ChronoSphere__WarpUnitsAtCell writes piVar6[0xa1]=ChronoReinfDelay — decompile_function 0x0065ec30 — STALE) | Constructor, ChronoSphere__WarpUnitsAtCell, TeleportLocomotionClass__StateMachineTick | HIGH       |
| 0x288       | [0xA2]  | int              | **ChronoDestCoordX** — destination X coord for chrono warp; set by ChronoSphere (corrected 2026-05-29: was WarpDestX; ChronoSphere__WarpUnitsAtCell writes piVar6[0xa2]=destX — decompile_function 0x0065ec30 — RTTI_LABEL_DRIFT) | Constructor, ChronoSphere__WarpUnitsAtCell        | HIGH       |
| 0x28C       | [0xA3]  | int              | **ChronoDestCoordY** — destination Y coord (corrected 2026-05-29: was WarpDestY — RTTI_LABEL_DRIFT) | Constructor, ChronoSphere__WarpUnitsAtCell        | HIGH       |
| 0x290       | [0xA4]  | int              | **ChronoDestCoordZ** — destination Z coord (corrected 2026-05-29: was WarpDestZ — RTTI_LABEL_DRIFT) | Constructor, ChronoSphere__WarpUnitsAtCell        | HIGH       |
| 0x294       | [0xA5]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x298       | [0xA6]  | byte (at +0)     | **IsHalfDamage** or similar flag — checked in Fire_At (halves ROF)      | Constructor, Fire_At                              | HIGH       |
| 0x29C       | [0xA7]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x2A0       | [0xA8]  | int              | **SpreadAttackIndex** — cycled 0-7 for spread weapons; used in Fire_At  | Constructor, Fire_At                              | HIGH       |
| 0x2A4       | [0xA9]  | byte (at +0)     | Unknown bool — initialized to 0                                         | Constructor                                       | MED        |
| 0x2A8       | [0xAA]  | int              | **IsMoving** or similar — checked in RockingUpdate for rocking behavior  | Constructor, RockingUpdate                        | HIGH       |
| 0x2AC       | [0xAB]  | int/ptr          | **MindController** ptr — who this unit is mind-controlling; checked in PerformDeploy | Constructor, PerformDeploy               | HIGH       |
| 0x2B0       | [0xAC]  | int/ptr          | **MindControllerHouse** or deploy state — initialized to 0; set in PerformDeploy | Constructor, PerformDeploy                | MED        |
| 0x2B4       | [0xAD]  | int/ptr          | **MindControlledBy** ptr — who is mind-controlling this unit; also used in Fire_At for ammo check | Constructor, Fire_At, Uncloak    | HIGH       |
| 0x2B8       | [0xAE]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x2BC       | [0xAF]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x2C0       | [0xB0]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x2C4       | [0xB1]  | byte (at +0)     | Unknown bool — initialized to 0                                         | Constructor                                       | MED        |
| 0x2C8       | [0xB2]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x2CC       | [0xB3]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x2D0       | [0xB4]  | int              | **LocomotorPtr** or related — checked != 0 in PerformDeploy             | Constructor, PerformDeploy                        | MED        |
| 0x2D4       | [0xB5]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x2D8       | [0xB6]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x2DC       | [0xB7]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x2E0       | [0xB8]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x2E4       | [0xB9]  | int              | **ArmorMultiplier** or similar — checked != 0 in Fire_At for range mod  | Constructor, Fire_At                              | MED        |
| 0x2E8       | [0xBA]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x2EC       | [0xBB]  | int              | **FireTimer.StartFrame** — set to g_CurrentFrameCounter on fire          | Constructor, Fire_At                              | HIGH       |
| 0x2F0       | [0xBC]  | int              | **FireTimer.Duration** — set with ROF value on fire                      | Fire_At                                           | HIGH       |
| 0x2F4       | [0xBD]  | int              | **FireTimer.Value** — set to ROF on fire                                 | Constructor, Fire_At                              | HIGH       |
| 0x2F8       | [0xBE]  | int              | **ROF** — set from GetROF vfunc result after firing                      | Constructor, Fire_At                              | HIGH       |
| 0x2FC       | [0xBF]  | int              | Unknown — initialized to -1 (0xFFFFFFFF)                                | Constructor                                       | MED        |
| 0x300       | [0xC0]  | int              | Unknown — initialized to 0                                              | Constructor                                       | MED        |
| 0x304       | [0xC1]  | int/ptr          | **AnimSlot1** (Charge anim) — checked/set in Fire_At for weapon anims    | Constructor, Fire_At                              | HIGH       |
| 0x308       | [0xC2]  | int/ptr          | **AnimSlot2** — checked/set in Fire_At for weapon anims                  | Constructor, Fire_At                              | HIGH       |
| 0x314       | [0xC5]  | int/ptr          | **AnimSlot3** (e.g. muzzle flash) — checked/set in Fire_At              | Constructor, Fire_At                              | HIGH       |
| 0x324       | [0xC9]  | int/ptr          | **RadBeam/WavePtr** — created for RadBeam/Sonic weapons in Fire_At       | Constructor, Fire_At                              | HIGH       |
| 0x328       | [0xCA]  | float            | **RockAngleX** (pitch) — rocking angle, float, used in RockingUpdate     | Constructor, RockingUpdate                        | HIGH       |
| 0x32C       | [0xCB]  | float            | **RockAngleY** (roll) — rocking angle, float, used in RockingUpdate      | Constructor, RockingUpdate                        | HIGH       |
| 0x330       | [0xCC]  | float            | **RockVelocityX** — angular velocity for pitch                           | Constructor, RockingUpdate                        | HIGH       |
| 0x334       | [0xCD]  | float            | **RockVelocityY** — angular velocity for roll                            | Constructor, RockingUpdate                        | HIGH       |
| 0x338       | [0xCE]  | int              | Unknown — initialized to -1 (0xFFFFFFFF)                                | Constructor                                       | MED        |

### Timer Group (0x2EC-0x2F8) — Firing Rate Timer
These three fields form a `RateTimer` for weapon firing:
- 0x2EC: Frame at which fire timer started
- 0x2F0: ROF duration value
- 0x2F4: Current ROF countdown value

### Byte-Level Fields (direct byte offsets via `(int)param_1 + 0xNNN`)

| Byte Offset | Type  | Purpose / Usage                                                              | Function(s)                           | Confidence |
|-------------|-------|------------------------------------------------------------------------------|---------------------------------------|------------|
| 0x082       | byte  | **INHERITED (ObjectClass)** — veteran/elite status byte                      | Fire_At, SelectWeaponAgainst          | HIGH       |
| 0x083       | byte  | **INHERITED (ObjectClass)** — flag checked after firing for suicide logic    | Fire_At                               | MED        |
| 0x142       | byte  | NOTE: This is TechnoTypeClass offset, not TechnoClass. IsAbductor on type.   | SelectWeaponAgainst (reads from type) | MED        |
| 0x150       | byte  | NOTE: This is TechnoTypeClass offset, not TechnoClass. IsLandTargeting.      | Fire_At (reads from type)             | MED        |
| 0x269       | byte  | Unknown bool                                                                 | Constructor                           | LOW        |
| 0x271       | byte  | Unknown bool                                                                 | Constructor                           | LOW        |
| 0x272       | byte  | Unknown bool                                                                 | Constructor                           | LOW        |
| 0x3CA       | short | Unknown 2-byte field — initialized to 0                                      | Constructor                           | LOW        |
| 0x3CD       | byte  | **IsRocking** flag — triggers rocking behavior in RockingUpdate              | Constructor, RockingUpdate            | HIGH       |
| 0x3CE       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x3CF       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x3D0       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x3D1       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x3D2       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x3D3       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x3D4       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x3D5       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x418       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x419       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x41A       | byte  | **IsFiringPrimary** — checked in Fire_At for LOS/shroud fire checks         | Constructor, Fire_At                  | HIGH       |
| 0x41B       | byte  | **IsFiringSecondary** — checked in Fire_At for LOS/shroud fire checks       | Constructor, Fire_At                  | HIGH       |
| 0x41C       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x41D       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x41E       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x41F       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x420       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x421       | byte  | **bool** — initialized to 1; checked in RockingUpdate for ship-specific rocking | Constructor, RockingUpdate         | HIGH       |
| 0x422       | byte  | **bool** — initialized to 1                                                 | Constructor                           | LOW        |
| 0x423       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x424       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x425       | byte  | **IsShipRocking** — enables ship-type rocking with velocity in RockingUpdate | Constructor, RockingUpdate            | HIGH       |
| 0x426       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x427       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x430       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x431       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x432       | byte  | **SuicideFlag** — set to 1 after self-destruct weapon fires on infantry      | Fire_At                               | MED        |
| 0x438       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x439       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x43A       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x44C       | byte  | **bool** — initialized to 1                                                 | Constructor                           | LOW        |
| 0x44D       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x465       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x47D       | byte  | Unknown bool — initialized to 0                                             | Constructor                           | LOW        |
| 0x661       | byte  | **IsMindControlled** or similar — checked in SelectWeaponAgainst for infantry | SelectWeaponAgainst                  | MED        |
| 0x6AD       | byte  | Unknown — set to 1 during PerformDeploy                                     | PerformDeploy                         | MED        |
| 0x6B5       | byte  | Checked in RockingUpdate for roll limit adjustment (building/deployed)       | RockingUpdate                         | MED        |
| 0x6B6       | byte  | Unknown — set to 1 during PerformDeploy                                     | PerformDeploy                         | MED        |
| 0x6CA       | byte  | Unknown — checked in SelectWeaponAgainst for infantry secondary weapon       | SelectWeaponAgainst                   | MED        |

### Additional Byte-Level Fields Found in Uncloak and Fire_At

| Byte Offset | Type  | Purpose / Usage                                                              | Function(s)                           | Confidence |
|-------------|-------|------------------------------------------------------------------------------|---------------------------------------|------------|
| 0x081       | byte  | Unknown — checked != 0 in Fire_At on target to skip fire                     | Fire_At                               | MED        |
| 0x2B4       | int   | **MindControlledBy** ptr — in Uncloak, iterates all technos checking if `[0x2B4] == this` | Uncloak                   | HIGH       |
| 0x2B0       | int   | **MindController** ptr — set during PerformDeploy, paired with 0x2B4        | PerformDeploy                         | HIGH       |

### Higher-Index Fields (0x2EC+)

| Byte Offset | Index    | Type             | Purpose / Usage                                                        | Function(s)                           | Confidence |
|-------------|----------|------------------|------------------------------------------------------------------------|---------------------------------------|------------|
| 0x3B8       | [0xEE]   | int              | **CurrentBurstIndex** — incremented per shot, modulo'd by Burst count  | Constructor, Fire_At, GetFLH          | HIGH       |
| 0x3BC       | [0xEF]   | int              | **TrailTimer.Start** — set to g_CurrentFrameCounter for muzzle flash   | Constructor, Fire_At                  | HIGH       |
| 0x3C0       | [0xF0]   | int              | **TrailTimer.Duration**                                                | Fire_At                               | HIGH       |
| 0x3C4       | [0xF1]   | int              | **TrailTimer.Value** — initialized to 0                                | Constructor, Fire_At                  | MED        |
| 0x3C8       | [0xF2]   | short            | **TurretFacing** or similar — set from function return                 | Constructor                           | MED        |
| 0x3CC       | [0xF3]   | byte (at +0)     | Unknown bool — initialized to 0                                       | Constructor                           | LOW        |
| 0x3D8       | [0xF6]   | int              | **PrimaryBurstRate.Total** — burst rate numerator; init 2              | Constructor, Fire_At                  | HIGH       |
| 0x3DC       | [0xF7]   | int              | **PrimaryBurstRate.Count** — burst rate denominator; init 1            | Constructor, Fire_At                  | HIGH       |
| 0x3E0       | [0xF8]   | int              | **PrimaryBurstRate.Current** — current burst; init 1                   | Constructor, Fire_At                  | MED        |
| 0x3E4       | [0xF9]   | int              | Unknown — initialized to 1                                            | Constructor                           | LOW        |
| 0x3E8       | [0xFA]   | int              | **PrimaryBurstRate.Ratio** — computed F6/FD                            | Fire_At                               | HIGH       |
| 0x3EC       | [0xFB]   | int              | Unknown — initialized to 0                                            | Constructor                           | LOW        |
| 0x3F0       | [0xFC]   | int              | **PrimaryBurstRate.Active** — set to 1 when burst fires                | Constructor, Fire_At                  | HIGH       |
| 0x3F4       | [0xFD]   | int              | **PrimaryBurstRate.Divisor** — clamped >= 1                            | Fire_At                               | HIGH       |
| 0x3F8       | [0xFE]   | int              | **SecondaryBurstRate.Total** — init 2                                  | Constructor, Fire_At                  | HIGH       |
| 0x3FC       | [0xFF]   | int              | **SecondaryBurstRate.Count** — init 1                                  | Constructor, Fire_At                  | HIGH       |
| 0x400       | [0x100]  | int              | **SecondaryBurstRate.Current** — init 1                                | Constructor                           | MED        |
| 0x404       | [0x101]  | int              | Unknown — initialized to 1                                            | Constructor                           | LOW        |
| 0x408       | [0x102]  | int              | **SecondaryBurstRate.Ratio** — computed FE/105                         | Fire_At                               | HIGH       |
| 0x40C       | [0x103]  | int              | Unknown — initialized to 0                                            | Constructor                           | LOW        |
| 0x410       | [0x104]  | int              | **SecondaryBurstRate.Active** — set to 1 when burst fires              | Fire_At                               | HIGH       |
| 0x414       | [0x105]  | int              | **SecondaryBurstRate.Divisor** — clamped >= 1                          | Fire_At                               | HIGH       |
| 0x418       | [0x106]  | byte (at +0)     | **CanCloak** flag — checked in Cloak helper (FUN_006f4a70)             | FUN_006f4a70                          | HIGH       |
| 0x428       | [0x10A]  | int              | Unknown — initialized to 0                                            | Constructor                           | LOW        |
| 0x42C       | [0x10B]  | int              | Unknown — initialized to 0                                            | Constructor                           | LOW        |
| 0x434       | [0x10D]  | int              | **BuildingAnim handle** — set from building offset after weapon fire   | Fire_At, Constructor                  | MED        |
| 0x43C       | [0x10F]  | int              | **BarrelRotationIndex** — cycled in Fire_At for multi-barrel weapons   | Constructor, Fire_At                  | HIGH       |
| 0x440       | [0x110]  | ptr (vtable*)    | **ILocomotor interface ptr** — vtable for FlashTimer or similar        | Constructor                           | MED        |
| 0x444       | [0x111]  | int              | Unknown — initialized to 0                                            | Constructor                           | LOW        |
| 0x448       | [0x112]  | int              | Unknown — initialized to 0                                            | Constructor                           | LOW        |
| 0x450       | [0x114]  | int              | Unknown — initialized to 0                                            | Constructor                           | LOW        |
| 0x454       | [0x115]  | int              | Unknown — initialized to 10                                           | Constructor                           | LOW        |
| 0x458       | [0x116]  | ptr (vtable*)    | **Timer/Flash vtable 2**                                               | Constructor                           | MED        |
| 0x460       | [0x118]  | int              | Unknown — initialized to 0                                            | Constructor                           | LOW        |
| 0x464       | [0x119]  | byte (at +0)     | Unknown bool — initialized to 1                                       | Constructor                           | LOW        |
| 0x468       | [0x11A]  | int              | Unknown — initialized to 0                                            | Constructor                           | LOW        |
| 0x470       | [0x11C]  | ptr (vtable*)    | **Timer/Flash vtable 3**                                               | Constructor                           | MED        |
| 0x474       | [0x11D]  | int              | **TargetList.Data** — pointer to target array for multi-target weapons | Fire_At                               | HIGH       |
| 0x478       | [0x11E]  | int              | **TargetList.Count** — current count of targets in list                | Fire_At, Constructor                  | HIGH       |
| 0x480       | [0x120]  | int              | **TargetList.CurrentIndex** — used and incremented in Fire_At          | Fire_At, Constructor                  | HIGH       |
| 0x484       | [0x121]  | int              | **TargetList.Capacity** — initialized to 10                            | Constructor                           | MED        |
| 0x48C       | [0x123]  | —                | Padding/gap                                                            | —                                     | —          |
| 0x49C       | [0x127]  | int              | Unknown — initialized to 1                                            | Constructor                           | LOW        |
| 0x4A0       | [0x128]  | int              | Unknown — initialized to 0                                            | Constructor                           | LOW        |
| 0x4B8       | [0x12E]  | byte (at +0)     | Unknown bool — initialized to 0                                       | Constructor                           | LOW        |
| 0x4BC       | [0x12F]  | int              | Unknown — initialized to 0                                            | Constructor                           | LOW        |
| 0x4D4       | [0x135]  | byte (at +0)     | Unknown bool — initialized to 0                                       | Constructor                           | LOW        |
| 0x4D8       | [0x136]  | int              | Unknown — initialized to 0                                            | Constructor                           | LOW        |
| 0x4F8       | [0x13E]  | byte (at +0)     | Unknown bool — initialized to 0                                       | Constructor                           | LOW        |
| 0x4FC       | [0x13F]  | int              | Timer — set to g_CurrentFrameCounter                                  | Constructor                           | MED        |
| 0x500       | [0x140]  | int              | Unknown — initialized to 0                                            | Constructor                           | LOW        |
| 0x504       | [0x141]  | int              | Unknown — initialized to 0                                            | Constructor                           | LOW        |
| 0x4F0       | [0x13C]  | int              | Unknown — initialized to -1 (0xFFFFFFFF)                              | Constructor                           | LOW        |
| 0x4F4       | [0x13D]  | int              | Unknown — initialized to -1 (0xFFFFFFFF)                              | Constructor                           | LOW        |
| 0x510       | [0x144]  | int              | Unknown — initialized to 0                                            | Constructor                           | LOW        |
| 0x514       | [0x145]  | int              | Unknown — initialized to 0                                            | Constructor                           | LOW        |
| 0x518       | [0x146]  | int              | Unknown — initialized to 0                                            | Constructor                           | LOW        |
| 0x51C       | [0x147]  | int              | Unknown — initialized to 0                                            | Constructor                           | LOW        |
| 0x520       | [0x148]  | int/ptr          | **BuildingTypeClass ptr** — used in Fire_At for IsWestwood check       | Fire_At, SelectWeaponAgainst          | HIGH       |

### Fields Referenced in PerformDeploy and Fire_At (BuildingClass range, 0x500+)

| Byte Offset | Index    | Type      | Purpose                                                              | Function(s)               | Confidence |
|-------------|----------|-----------|----------------------------------------------------------------------|---------------------------|------------|
| 0x5D4       | [0x175]  | int       | **BuildingPtr** or similar — checked != 0 in PerformDeploy/Fire_At  | PerformDeploy, Fire_At    | MED        |
| 0x658       | [0x196]  | —         | —                                                                    | —                         | —          |
| 0x664       | [0x199]  | int       | Unknown — checked > 0 in PerformDeploy for deploy animation          | PerformDeploy             | MED        |
| 0x674       | [0x19D]  | int/ptr   | **ILocomotor** COM interface ptr — used in GetFLH, PerformDeploy     | GetFLH, PerformDeploy     | HIGH       |
| 0x694       | [0x1A5]  | int/ptr   | **LinkedUnit** ptr — checked in PerformDeploy                        | PerformDeploy             | MED        |
| 0x698       | [0x1A6]  | int       | **TempInvulTimer** — set to g_CurrentFrameCounter + 0x14 in Fire_At | Fire_At                   | MED        |
| 0x69C       | [0x1A7]  | int       | **MultiBarrelIndex** — incremented/modulo'd for multi-barrel weapons | Fire_At                   | HIGH       |

### Embedded Objects (constructed via function calls in constructor)

| Byte Offset Range | Purpose                                                    | Confidence |
|-------------------|------------------------------------------------------------|------------|
| 0x150-0x163       | Embedded object — initialized via `FUN_0074ff30()`         | MED        |
| 0x304-0x337       | Embedded objects — initialized via `FUN_006c95e0()` etc.   | MED        |
| 0x304-0x31F       | Array region — 8 dwords zeroed in a loop (0xC1-0xC8)      | HIGH       |

## Summary Statistics

- Total unique offsets discovered: ~150+
- Offsets with HIGH confidence naming: ~35
- Offsets with identified purpose: ~60
- Approximate struct size: 0x69C+ bytes (at least 1692 bytes), extends further in subclasses

## Key Functional Groups

### Weapon System (HIGH confidence)
- `[0x4E]` (0x138): ForcedWeaponIndex (-1 = none)
- `[0x50]` (0x140): CurrentWeaponIndex
- `[0xEE]` (0x3B8): CurrentBurstIndex (incremented per shot)
- `[0x10F]` (0x43C): BarrelRotationIndex (multi-barrel cycling)
- `[0xBB-0xBE]` (0x2EC-0x2F8): Fire rate timer (start frame, duration, value, ROF)
- `[0xC1]` (0x304): Charge animation pointer
- `[0xC2]` (0x308): Secondary animation pointer
- `[0xC5]` (0x314): Muzzle flash animation pointer
- `[0xC9]` (0x324): RadBeam/Wave/Sonic weapon pointer
- `[0xF6-0xFD]` (0x3D8-0x3F4): Primary weapon burst rate data (7 fields)
- `[0xFE-0x105]` (0x3F8-0x414): Secondary weapon burst rate data (7 fields)

### Rocking/Physics (HIGH confidence)
- `[0xCA]` (0x328): RockAngleX (float, pitch)
- `[0xCB]` (0x32C): RockAngleY (float, roll)
- `[0xCC]` (0x330): RockVelocityX (float)
- `[0xCD]` (0x334): RockVelocityY (float)
- `0x3CD`: IsRocking flag (byte)
- `0x425`: IsShipRocking flag (byte)
- `0x421`: Ship rocking enable (byte, init 1)

### Cloaking (HIGH confidence)
- `[0x87]` (0x21C): Owner (HouseClass ptr)
- `[0x88]` (0x220): CloakState (enum: 0=uncloaked, 2=cloaked)
- `[0x106]` (0x418): CanCloak flag (byte)

### Chrono/Warp
- `[0x95-0x97]` (0x254-0x25C): Chrono **source** coordinates (X, Y, Z) — position before warp (corrected 2026-05-29: were labelled "Chrono destination")
- `0x270` (WarpingOut), `0x271` (BeingWarped), `0x27C` (ChronoInTransit): warp state flags (corrected 2026-05-29: were labelled "Unknown bool")
- `[0xA0]` (0x280): PendingWarpPhase — set to 3 by ChronoSphere (corrected 2026-05-29: was "Unknown")
- `[0xA1]` (0x284): ChronoLockDuration — set to ChronoReinfDelay (corrected 2026-05-29: was "Unknown")
- `[0xA2-0xA4]` (0x288-0x290): Chrono **destination** coordinates (X, Y, Z) (corrected 2026-05-29: were labelled "WarpDest")

### Mind Control
- `[0xAB]` (0x2AC): MindController ptr (who this unit is controlling)
- `[0xAD]` (0x2B4): MindControlledBy ptr (who is controlling this unit)

### Deploy / Locomotor
- `[0xAC]` (0x2B0): Deploy state or MindControllerHouse
- `[0x19D]` (0x674): ILocomotor COM interface
- `[0x1A5]` (0x694): Linked unit pointer

### Target List (multi-target weapons)
- `[0x11D]` (0x474): TargetList data pointer
- `[0x11E]` (0x478): TargetList count
- `[0x120]` (0x480): TargetList current index
- `[0x121]` (0x484): TargetList capacity (init 10)

---

## Tier 6 application record (2026-08-17, Claude Code session)

Corridor: `docs/plans/2026-08-17-ghidra-typing-corridor-program.md` row 6, "TechnoClass sim
fields". Snapshot before mutations:
`C:/Users/enok/Documents/ghidra-backups/2026-08-17-pre-tier6` (17 files, 243,359,753 bytes,
byte-count verified with the program closed). Live Ghidra is the authority on applied-ness.

Re-rank check: the goal stream is unchanged (Phase 5; PR #131 still open as a draft; recent
merges are architecture/domain-boundary work). Corridor order kept, no re-rank.

**Structure size unchanged at 1312 bytes (0x520).** The live `/RA2/TechnoClass` is a flat
YRpp-style import that does NOT embed its bases — it repeats the ObjectClass rows (Health 0x6C,
OnBridge 0x8C, IsAlive 0x90, Location 0x9C-0xA4) and the MissionClass row at 0xC4 inline.
Contrast `/MissionClass`, which does embed `ObjectClass` at offset 0. Reconciling the two
shapes is a bounded residual for a later tier, not attempted here.

Inheritance boundary re-derived from the constructor chain this session: `TechnoClass__Constructor`
0x006F2B40 calls `RadioClass__Constructor` 0x0065A750 -> `MissionClass__Constructor` 0x005B2DA0 ->
`ObjectClass__Constructor` 0x005F3900, and TechnoClass's own first store is at +0xF0. So
everything below 0xF0 belongs to a base class.

### Rows applied (9)

| Offset | Change | Evidence |
|---|---|---|
| 0x70 | `nSmoothedHealth` -> **`EstimatedHealth`** | REFUTED as a display value: it is allowed to go negative and is floored at -30 (`ADD EAX,0x1E; TEST; JGE; MOV [ESI+0x70],0xFFFFFFE2` at 0x006F9F8E-0x006F9F9C), attackers pre-deduct predicted damage into it (`SUB [EDI+0x70],EAX` at 0x006FE622 and 0x007099B5, EAX from the veterancy/armor damage calculator 0x006FDB80), and the consumer is target selection, not rendering — `Evaluate_Candidate` rejects a candidate when it goes non-positive (0x006F7CF7) and scales the score by `estimate / Strength` (0x006F872A-0x006F874B). `DrawHealthBar` contains no access to it at all. Anti-overkill bookkeeping, and it now matches the name proven for the same offset in ObjectClass. |
| 0x150 | `int Veterancy` -> **`float`** | Root-verified this session: `VeterancyClass__IsVeteran` 0x0074FF90 is `FLD float ptr [ECX]; FCOMP float ptr [0x007E2AC8]; ...; FLD float ptr [ECX]; FCOMP float ptr [0x007E37B4]` — a 4-byte float load. `read_memory 0x007E2AC8` = `00 00 80 3F` = **1.0f**; `read_memory 0x007E37B4` = `00 00 00 40` = **2.0f**. Bands: rookie [0,1), veteran [1,2), elite >=2. It accumulates — 0x0074FF50 (called from `RecordKill`) does `FADD`/`FST float ptr [ECX]`. The applied `int` was wrong. |
| 0x21C | `pointer` -> **`HouseClass *`** | Root-verified: `0x006F9DC0` is `MOV EAX,dword ptr [ECX+0x21C]; RET` — the owner getter, tied to a vtable slot by 0x007079C2 (`CALL [EAX+0x3C]`) followed immediately by 0x007079C5 (`CMP EAX,[ESI+0x21C]`). **Decisive proof is `TechnoClass__ChangeOwner` 0x007014A0**, which takes the new house in EBP (0x007014A4), early-outs when it already matches (0x007014AB/0x007014B1), and writes it straight in at **0x00701735 `MOV dword ptr [ESI+0x21C],EBP`** — with symmetric house-side bookkeeping using the old pointer as receiver at 0x007015D8 and the new one at 0x007015E4. It then indexes the house arrays that `HouseClass__Constructor` zero-initialises at +0x53E4 / +0x5438 (0x004F62A1, 0x004F62AE) via 0x00701646 / 0x00701622. |
| 0x51C | `pointer` -> **`HouseClass *`** | Root-verified and decisive: at 0x006F421C-0x006F4229 `TechnoClass__Init_Managers` does `MOV EAX,dword ptr [ESI+0x21c]` then `MOV dword ptr [ESI+0x51c],EAX` — 0x51C is assigned directly FROM Owner, so both are the same pointee class. |
| 0x2AC | `LocomotorTarget` -> **`DeployedFrom`** | REFUTED as a target. It is one half of a bidirectional deploy link: `PointerExpired` 0x007077C0 proves the invariant — clearing this object's 0x2AC also zeroes the other object's 0x2B0, and vice versa. `PerformDeploy` establishes the pair in one store run (`MOV [EBX+0x2ac],EAX` 0x00710307, `MOV [EAX+0x2b0],EBX` 0x00710318). The accessors are `BuildingClass__CanSellOrUndeploy` and the deploy/undeploy family, not locomotion. |
| 0x2B0 | **NEW `DeployedInto`** | Forward half of the same link (see above). |
| 0x2B8 | **NEW `SuspendedTarget`** | `TechnoClass__Override_Mission` 0x007013A0 does 0x2B8 <- 0x2B4; `TechnoClass__Restore_Mission` 0x007013E0 re-issues `Assign_Target` from it. Exact mirror of the FootClass NavCom/SuspendedNavCom pair at 0x5A4/0x5A8. |
| 0xC4 | `nAITickCounter` -> **`MissionAITickCount`** | Scope was wrong, not the width. `MissionClass__Constructor` 0x005B2DD1 zeroes it; it is incremented once per AI tick immediately before `Mission_Dispatch` (0x006FA646-0x006FA655); and it is **reset to 0 on every mission change** by `Assign_Mission` 0x005B3010 and `Commence` 0x005B35B9. It is per-mission elapsed ticks, and it is a MissionClass field — `/MissionClass` already names the same offset 196 `dwMissionAITickCount`, so the two structs now agree. |
| 0x1D8 | **NEW `IsDisguised`** (bool) | `TechnoClass__IsDisguised_Getter` 0x0041C020 (vtable +0xC8) returns the byte at +0x1D8; `Init_Managers` sets it at 0x006F4222. |

### Holes — recorded, not guessed

| Offset | Why it stays a hole |
|---|---|
| 0x154 | Alignment padding so the 8-byte double at 0x158 is 8-aligned. The 4-byte width of 0x150 (proven above) independently corroborates it. |
| 0x184 | 0x180/0x184/0x188 are ONE 12-byte timer, not two independent ints: `0x0070F770` does `LEA ESI,[ECX+0x180]` then three consecutive dword stores (0x0070F7C1-0x0070F7CA). The inlined read pattern (0x006FA65A-0x006FA677) touches only +0x00 (StartTime) and +0x08 (DelayTime) — 0x184 is written and never read. Reload values come from RulesClass +0xE04 `GuardAreaTargetingDelay` and +0xE08 `NormalTargetingDelay`; ctor default 45. |
| 0x2C4 | A byte forming a second condition in `TechnoClass__IsMindControlled` 0x007105E0 (`0x2C0 != 0 || byte[0x2C4] != 0`). Writer not located. Consequence for porting: mind-control state is NOT a single pointer test. |
| 0x4FC | Four writers, all storing the global frame counter from `[0x00A8ED84]` (ctor 0x006F3106, 0x006FA6C9, 0x007094A9, 0x0070982E); no reader found. Superseded by the 0x180 timer. |
| 0x504 | `EMPLockRemaining` — type and role are right (`MOV EDX,[ECX+0x504]; TEST; SETG` at 0x0070EFD0; per-tick decrement in `AI_Update` 0x006FAF0D-0x006FAF24; command gate in `EventClass__Execute` 0x004C7701) but **DORMANT**: the only writer is `EMPulseClass__Apply`, reachable only from the constructor 0x004C52B0, which has no cross-references at all; the sibling ctor 0x004C5370 is reached only from a COM `IClassFactory::CreateInstance` stub (savegame deserialization). Nothing in the shipping binary can set it non-zero during play. TS Firestorm residue — do NOT implement as default, per the ENGINE.md TS-legacy rule. |

### Corrections this tier makes to prior claims

- The chrono cluster is 0x271 (`BeingWarped` byte), 0x27C (warp-pending byte), 0x280 (phase
  dword), 0x284 (delay from Rules+0xBF0), 0x288-0x290 (destination triple). **0x218 is not
  touched by any chrono path**, and **0x6AF cannot be a TechnoClass field** — the class ends at
  0x520, so that offset is FootClass or deeper. Docs asserting either are wrong.
- `TechnoClass__ClearChronoFields` 0x00720440 — **receiver REFUTED**. It writes offset 0 as a
  byte and treats 0x288-0x28A as bytes, so its receiver is not a TechnoClass. Its own plate
  comment already admits it has no callers. Not valid evidence for 0x288. Label correction
  deferred to a tier that owns that address.
- 0x304-0x323 is an **8-pointer array**, not five unrelated fields: the constructor zeroes
  exactly eight dwords and `PointerExpired` clears them in a matching 8-iteration loop.
  `DamageSparkSystem` 0x308 is one of three particle-system slots (0x304, 0x308, 0x314),
  each gated by a different weapon flag.
- `Ammo` 0x2FC initialises to **-1**, the unlimited sentinel — which is why `GetFireError`
  tests `== 0` rather than `<= 0`. In stock YR only aircraft consume it.
- `IsSinking` 0x3CD is correct but the name is too narrow: four distinct writers set it and
  only one is a ship (unit death 0x00737E51, jumpjet crash 0x0054CEB7, temporal erasure
  0x00629C69, failed chrono landing 0x0071896B/0x00718AC2). It means "dying / playing the
  sink-away", not "boat in water". Left as-is; recorded here.
- The four rocking floats 0x328-0x334 are **live, not TS-dormant**: the per-frame deltas are
  written by `DriveLocomotionClass__Process_Drive_Track` (0xBD4CCCCD = -0.05f at 0x004B19FA)
  and `ShipLocomotionClass__Process_Drive_Track` (0xBCA3D70A = -0.02f at 0x006A1090) — 4-byte
  IEEE-754 single patterns, an independent width proof that does not rely on FPU operands.

### Critic pass

A fresh read-only critic re-verified all four type decisions from raw bytes without the
applier's reasoning. **All four CONFIRMED**, three of them on evidence stronger than the
applier had:

- **0x150 float** — the ctor binding is a real callsite (`LEA ECX,[ESI+0x150]` 0x006F2BDC ->
  `CALL 0x0074FF30`, whose body is a 4-byte zero-init). Every accessor uses FPU 4-byte
  operands and never an integer form; the setter writes the immediate `0x40000000`, the
  IEEE-754 pattern for 2.0f. Constants re-read as 1.0f and 2.0f.
- **0x21C HouseClass\*** — see the row above; the critic found `ChangeOwner`'s direct store,
  which is much stronger than the indirect chain first cited.
- **0x51C HouseClass\*** — confirmed verbatim, two instructions apart on the same receiver.
- **0x2AC/0x2B0** — the reciprocal-clear invariant appears in `PointerExpired` in BOTH
  directions (0x00707B64-0x00707B85 and 0x00707B8B-0x00707BAC), each with a partner fixup and
  an unlink helper call. The critic checked every other pointer field cleared in that same
  function — 0x2D4, 0x2E0, 0x2CC, 0x278, 0x294, 0x428, 0x1D4, 0x2C8, 0x130, 0x2B4, 0x2B8,
  0x434, 0x12C — and **none** gets a partner fixup. The pair is structurally an owned
  bidirectional link, not a target reference; the old `LocomotorTarget` name was wrong.

Struct size re-verified at 1312 bytes by the critic.

A second fresh critic audited all five holes. **None refuted.** It worked from byte-pattern
sweeps over every store/load encoding of each displacement rather than operand-text searches,
which is the method that avoids the false negatives this program has hit twice.

- **0x504 dormancy — doubly sealed, not merely unrefuted.** Full writer enumeration over every
  store encoding of displacement 0x504 yields exactly three sites: the constructor's zero-init
  (0x006F3112, EBX zeroed at 0x006F2B4C), and the two non-zero setters inside
  `EMPulseClass__Apply` (0x004C5718, 0x004C5829). `get_function_callers` and `get_xrefs_to` on
  `Apply` return only `EMPulseClass__Constructor` 0x004C52B0, and `get_xrefs_to 0x004C52B0`
  returns nothing. Beyond that, a byte search for the little-endian address constants
  `B0 52 4C 00` and `E0 54 4C 00` finds **no match anywhere in the image** — so neither
  function appears in any vtable, dispatch table, or `PUSH imm32`. The sibling ctor 0x004C5370
  is the blank one (four vtable pointers, no `Apply` call) and its only xref is an
  `IClassFactory::CreateInstance` chunk at 0x006BFB10 — decoded and confirmed by its
  `E_INVALIDARG` / `CLASS_E_NOAGGREGATION` / `E_OUTOFMEMORY` constants — i.e. deserialization
  only. The reader, the decrement and the `EventClass__Execute` gate are live code that will
  always observe 0. Stated residual: a bulk struct copy or a rebased-pointer write would evade
  a displacement-keyed sweep, but neither creates a value.
- **0x4FC** — the four stores are exactly the claimed sites and nothing else; load, compare and
  address-taken sweeps (`8B`, `3B`, `39`, `8D` forms) return **no match program-wide**.
- **0x180/0x184/0x188** — three consecutive dword stores through one base
  (`LEA ESI,[ECX+0x180]` at 0x0070F783, stores at 0x0070F7C1/0x0070F7C7/0x0070F7CA), read
  pattern touches only +0x00 and +0x08. Extra corroboration the applier did not have: both
  reload sites pass **`LEA EAX,[ESI+0x180]`** — the block base — to a helper, so the timer is
  addressed as an object and never as loose ints.
- **0x154** — `0x00701957` is byte-for-byte `DC 8E 58 01 00 00` = `FMUL m64fp`, and 0x160 is
  `DC 8E 60 01 00 00`; both are genuinely 8-byte doubles, so the padding and the layout hold.
- **0x2C4** — byte width and the OR shape confirmed verbatim. Writer still UNSETTLED. Worth
  flagging: this field has **the same shape as 0x504** — a flag read in two places
  (`IsMindControlled` and `Evaluate_Candidate` 0x006F8380), zero-initialised by the
  constructor, and never observed being set. It may be a second dormant path; that is a
  bounded question for whichever tier next touches mind control.

Tier 6 refutation rate: **0 of 9 applied rows and 0 of 5 holes**.

Carried UNSETTLED items, honestly labelled: the five veterancy accessors are tied to +0x150 by
method-cluster co-location rather than a per-accessor verified callsite (the ctor binding is
proven); the exact vtable slot index of 0x006F9DC0 was corroborated indirectly rather than
computed from a vtable base; and in `PerformDeploy` the EBX operand was not traced back to its
allocation, so the link *direction* is proven but the crispness of the two field names rests on
the function's identity.

### Method traps recorded this tier

- `CALL dword ptr [reg+0xNNN]` is a **vtable slot dispatch**, not a field access. Offset sweeps
  must exclude CALL forms — this produced apparent 0x2AC/0x2B8/0x3B8 "accesses" that are not.
- A field can be reached through `LEA`/`ADD reg,0xNNN` on the aggregate base, which **no**
  operand-pattern search on `[reg+0xNNN]` will ever match. The real consumer of the 0x288
  triple reads it via `ADD ECX,0x288`. Any "no reader" hole must account for this.
- `search_instructions` program-wide sweeps with `+ 0xNN]` operand patterns are unreliable
  (the filter normalizes `+` spacing) and hung one critic agent to the point of being killed.
  Prefer `get_xrefs_to` and function-scoped searches.
