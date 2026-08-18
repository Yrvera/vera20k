# HouseClass Constructor (0x4f54a0) -- Complete Field-by-Field Analysis

Source: Ghidra decompilation of `gamemd.exe` function at 0x004f54a0
Size: 4250 bytes of code. Creates a ~90KB object (0x160B8 bytes).

---

## CRITICAL: param_1 Type Analysis

```c
undefined4 * __thiscall FUN_004f54a0(undefined4 *param_1, int param_2)
```

**param_1 is `undefined4 *`** (pointer to 4-byte values). This means:
- `param_1[0x26]` = byte offset `0x26 * 4` = **0x98**
- `param_1[0x5829]` = byte offset `0x5829 * 4` = **0x160A4**
- `*(int *)((int)param_1 + 0x1ED)` = **direct byte offset 0x1ED** (cast to int first)

All `param_1[N]` indices below are converted to **byte offsets = N * 4**.

param_2 is `int` -- this is the `HouseTypeClass*` (CountryTypeClass) pointer.

---

## Phase 1: Base Class Initialization (AbstractClass)

```c
FUN_00410170();  // AbstractClass::AbstractClass() -- base class constructor
```

This sets up the AbstractClass base fields (UniqueID, RTTI type, etc.) at offsets +0x00..+0x2C.

---

## Phase 2: HouseIndex and HouseType Pointer

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `param_1[0xc] = 0xffffffff` | 0x0C | **+0x30** | -1 | HouseIndex (unassigned sentinel) |
| `param_1[0xd] = param_2` | 0x0D | **+0x34** | HouseTypeClass* | Pointer to CountryTypeClass |

---

## Phase 3: DynamicVectorClass Array Initialization (13 DVCs)

Each DynamicVectorClass is 0x18 bytes (6 dwords):
```
+0x00: vtable pointer
+0x04: data pointer (Buffer*)
+0x08: capacity
+0x09: owns_memory (byte inside this dword)
+0x0C: unused/padding
+0x10: count
+0x14: grow_amount
```

The init functions (`FUN_00510640`, `FUN_00510550`, `FUN_00510690`, `FUN_00510500`,
`FUN_005106e0`, `FUN_00510780`, `FUN_005105f0`) are constructors for different
DynamicVectorClass template instantiations.

### DVC #1: Special type (FUN_00510640)

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `FUN_00510640(0,0)` | -- | -- | -- | Constructs DVC at +0x38 |
| `param_1[0xe] = &PTR_FUN_007ea5a4` | 0x0E | **+0x38** | vtable | DVC vtable |
| `param_1[0x13] = 10` | 0x13 | **+0x4C** | 10 | grow_amount |
| `param_1[0x12] = 0` | 0x12 | **+0x48** | 0 | count |

### DVCs #2-#12: Standard type (FUN_00510550), all with PTR_FUN_007e9e24 vtable

Each initialized identically: vtable, count=0, grow_amount=10.

| DVC # | Vtable Index | Byte Offset (vtable) | Count Index | Byte Offset (count) | Grow Index | Byte Offset (grow) |
|-------|-------------|---------------------|-------------|---------------------|------------|---------------------|
| 2 | 0x14 | **+0x50** | 0x18 | +0x60 | 0x19 | +0x64 |
| 3 | 0x1A | **+0x68** | 0x1E | +0x78 | 0x1F | +0x7C |
| 4 | 0x20 | **+0x80** | 0x24 | +0x90 | 0x25 | +0x94 |
| 5 | 0x26 | **+0x98** | 0x2A | +0xA8 | 0x2B | +0xAC |
| 6 | 0x2C | **+0xB0** | 0x30 | +0xC0 | 0x31 | +0xC4 |
| 7 | 0x32 | **+0xC8** | 0x36 | +0xD8 | 0x37 | +0xDC |
| 8 | 0x38 | **+0xE0** | 0x3C | +0xF0 | 0x3D | +0xF4 |
| 9 | 0x3E | **+0xF8** | 0x42 | +0x108 | 0x43 | +0x10C |
| 10 | 0x44 | **+0x110** | 0x48 | +0x120 | 0x49 | +0x124 |
| 11 | 0x4A | **+0x128** | 0x4E | +0x138 | 0x4F | +0x13C |
| 12 | 0x50 | **+0x140** | 0x54 | +0x150 | 0x55 | +0x154 |

All use vtable `PTR_FUN_007e9e24`, count=0, grow_amount=10.

Known purposes:
- +0x68 (DVC #3): OwnedObjects (TechnoClass*[])
- +0x140 (DVC #12): OwnedUpgrades (BuildingClass*[])

---

## Phase 4: Scalar Fields After DVCs (+0x158..+0x17C)

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `param_1[0x56] = 0` | 0x56 | **+0x158** | 0 | SpySatCount |
| `param_1[0x57] = 0` | 0x57 | **+0x15C** | 0 | CloakDeviceCount |
| `param_1[0x58] = 0` | 0x58 | **+0x160** | 0 | Unknown (power-related?) |
| `param_1[0x59] = 0` | 0x59 | **+0x164** | 0 | PowerOutputUnits |
| `param_1[0x5a] = 0` | 0x5A | **+0x168** | 0 | PowerDrainUnits |

### DVC #13: Garrison structures (FUN_00510690)

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `FUN_00510690(0,0)` | -- | -- | -- | Constructs DVC at +0x16C |
| `param_1[0x5b] = &PTR_FUN_007ea944` | 0x5B | **+0x16C** | vtable | GarrisonStructures DVC |
| `param_1[0x60] = 10` | 0x60 | **+0x180** | 10 | grow_amount |
| `param_1[0x5f] = 0` | 0x5F | **+0x17C** | 0 | count |

---

## Phase 5: Difficulty Level and 7 Difficulty Doubles (+0x184..+0x1C0)

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `param_1[0x61] = *(DAT_00a8b230 + 0x610)` | 0x61 | **+0x184** | ScenarioClass+0x610 | DifficultyLevel; Scenario reset `0x00683610` defaults it to `1` (Normal) |
| `param_1[0x62] = 0` | 0x62 | **+0x188** | 0 | Firepower (double low dword) |
| `param_1[99] = 0x3ff00000` | 0x63 | **+0x18C** | 0x3FF00000 | Firepower (double high dword) |

The 7 doubles are stored as pairs of dwords (little-endian IEEE 754 double).
`0x3FF0000000000000` = **1.0 (double)**.

**Complete difficulty double layout:**

| Field | Low Index | High Index | Low Offset | High Offset | Default |
|-------|-----------|------------|------------|-------------|---------|
| Firepower | 0x62 | 0x63 | **+0x188** | +0x18C | 1.0 |
| Groundspeed | 0x64 | 0x65 | **+0x190** | +0x194 | 1.0 |
| Airspeed | 0x66 | 0x67 | **+0x198** | +0x19C | 1.0 |
| Armor | 0x68 | 0x69 | **+0x1A0** | +0x1A4 | 1.0 |
| ROF | 0x6A | 0x6B | **+0x1A8** | +0x1AC | 1.0 |
| Cost | 0x6C | 0x6D | **+0x1B0** | +0x1B4 | 1.0 |
| BuildTime | 0x6E | 0x6F | **+0x1B8** | +0x1BC | 1.0 |

Each double is stored as `[low_dword=0, high_dword=0x3FF00000]`.

The pseudocode shows:
```c
param_1[0x62] = 0;            // +0x188: Firepower low
param_1[99]   = 0x3ff00000;   // +0x18C: Firepower high  → 1.0
param_1[100]  = 0;            // +0x190: Groundspeed low
param_1[0x65] = 0x3ff00000;   // +0x194: Groundspeed high → 1.0
param_1[0x66] = 0;            // +0x198: Airspeed low
param_1[0x67] = 0x3ff00000;   // +0x19C: Airspeed high    → 1.0
param_1[0x68] = 0;            // +0x1A0: Armor low
param_1[0x69] = 0x3ff00000;   // +0x1A4: Armor high       → 1.0
param_1[0x6a] = 0;            // +0x1A8: ROF low
param_1[0x6b] = 0x3ff00000;   // +0x1AC: ROF high         → 1.0
param_1[0x6c] = 0;            // +0x1B0: Cost low
param_1[0x6d] = 0x3ff00000;   // +0x1B4: Cost high        → 1.0
param_1[0x6e] = 0;            // +0x1B8: BuildTime low
param_1[0x6f] = 0x3ff00000;   // +0x1BC: BuildTime high   → 1.0
```

**Note:** The existing report says offsets are +0x188 through +0x1C0 with RepairDelay at
+0x1C0 and BuildDelay at +0x1C8. The constructor only initializes 7 doubles (through +0x1BC).
RepairDelay (+0x1C0) and BuildDelay (+0x1C8) are set later by SetDifficulty, not here.

---

## Phase 6: Flags and State Fields (+0x1C0..+0x20C)

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `param_1[0x70] = 0` | 0x70 | **+0x1C0** | 0 | RepairDelay double (low) |
| `param_1[0x71] = 0` | 0x71 | **+0x1C4** | 0 | RepairDelay double (high) — 0.0 initially |
| `param_1[0x72] = 0` | 0x72 | **+0x1C8** | 0 | BuildDelay double (low) |
| `param_1[0x73] = 0` | 0x73 | **+0x1CC** | 0 | BuildDelay double (high) |
| `param_1[0x74] = 0` | 0x74 | **+0x1D0** | 0 | IQ level |
| `param_1[0x75] = 1` | 0x75 | **+0x1D4** | 1 | TechLevel (default 1) |
| `param_1[0x76] = 0` | 0x76 | **+0x1D8** | 0 | RadarShareBitfield |
| `param_1[0x77] = 0` | 0x77 | **+0x1DC** | 0 | StartingCredits |
| `param_1[0x78] = 0xffffffff` | 0x78 | **+0x1E0** | -1 | StartingEdge (-1 = none) |
| `param_1[0x79] = 0` | 0x79 | **+0x1E4** | 0 | AI build state |

### Byte-level flags (cast to byte via `*(undefined1*)`)

The pattern `*(undefined1 *)(param_1 + 0x7b) = 0` writes to byte offset **0x7B * 4 = 0x1EC**.
The pattern `*(undefined1 *)((int)param_1 + 0x1ed) = 0` writes to **direct byte offset 0x1ED**.

| Decompiled | Byte Offset | Value | Field |
|-----------|-------------|-------|-------|
| `param_1[0x7a] = ...` (set later) | **+0x1E8** | varies | SideIndex |
| `*(byte*)(param_1 + 0x7b) = 0` | **+0x1EC** | 0 | IsHumanPlayer |
| `*((int)param_1 + 0x1ed) = 0` | **+0x1ED** | 0 | PlayerControl |
| `*((int)param_1 + 0x1ee) = 0` | **+0x1EE** | 0 | AI active (IsAutoProduction) |
| `*((int)param_1 + 0x1ef) = 0` | **+0x1EF** | 0 | AI triggers active |
| `*(byte*)(param_1 + 0x7c) = 1` | **+0x1F0** | **1** | Unknown flag (default true) |
| `*((int)param_1 + 0x1f1) = 0` | **+0x1F1** | 0 | Unknown |
| `*((int)param_1 + 0x1f2) = 0` | **+0x1F2** | 0 | Unknown |
| `*((int)param_1 + 499)` → 0x1F3 | **+0x1F3** | 0 | HasBeenSpied |
| `*(byte*)(param_1 + 0x7d) = 0` | **+0x1F4** | 0 | Unknown |
| `*((int)param_1 + 0x1f5) = 0` | **+0x1F5** | 0 | IsDefeated |
| `*((int)param_1 + 0x1f6) = 0` | **+0x1F6** | 0 | IsLosing / FlagToWinPending |
| `*((int)param_1 + 0x1f7) = 0` | **+0x1F7** | 0 | HasWon |
| `*(byte*)(param_1 + 0x7e) = 0` | **+0x1F8** | 0 | HasLost |
| `*((int)param_1 + 0x1f9) = 0` | **+0x1F9** | 0 | Unknown flag |
| `*((int)param_1 + 0x1fa) = 0` | **+0x1FA** | 0 | Unknown flag |
| `*((int)param_1 + 0x1fb) = 0` | **+0x1FB** | 0 | NeedsRebuild |
| `*(byte*)(param_1 + 0x7f) = 1` | **+0x1FC** | **1** | ProductionChanged (default true) |
| `param_1[0x80] = 0` | 0x80 | **+0x200** | 0 | Unknown dword |
| `param_1[0x81] = 0` | 0x81 | **+0x204** | 0 | Unknown dword |
| `*(byte*)(param_1 + 0x82) = 0` | **+0x208** | 0 | Unknown byte |
| `param_1[0x83] = 0xffffffff` | 0x83 | **+0x20C** | -1 | CurrentFactoryIndex (-1 = none) |

---

## Phase 7: More Flag Bytes (+0x240..+0x24C)

| Decompiled | Byte Offset | Value | Field |
|-----------|-------------|-------|-------|
| `*(byte*)(param_1 + 0x90) = 0` | **+0x240** | 0 | Unknown |
| `*((int)param_1 + 0x241) = 0` | **+0x241** | 0 | Unknown |
| `*((int)param_1 + 0x242) = 0` | **+0x242** | 0 | Unknown |
| `*((int)param_1 + 0x243) = 0` | **+0x243** | 0 | Unknown |
| `*(byte*)(param_1 + 0x91) = 0` | **+0x244** | 0 | Unknown |
| `*((int)param_1 + 0x245) = 0` | **+0x245** | 0 | Unknown |
| `*((int)param_1 + 0x246) = 0` | **+0x246** | 0 | AnnounceReadyFlag |
| `*((int)param_1 + 0x247) = 0` | **+0x247** | 0 | Unknown |
| `*(byte*)(param_1 + 0x92) = 0` | **+0x248** | 0 | Unknown |
| `*((int)param_1 + 0x249) = 0` | **+0x249** | 0 | Unknown |
| `*((int)param_1 + 0x24a) = 0` | **+0x24A** | 0 | Unknown |
| `*((int)param_1 + 0x24b) = 1` | **+0x24B** | **1** | SidebarUpdatePending (default true) |
| `param_1[0x93] = param_1[0x74]` | 0x93 | **+0x24C** | = IQ (0) | CurrentIQ (copies IQ field) |
| `param_1[0x94] = 0` | 0x94 | **+0x250** | 0 | Unknown dword |

---

## Phase 8: SuperWeapons DVC (FUN_00510500)

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `FUN_00510500(0,0)` | -- | -- | -- | Constructs DVC at +0x254 |
| `param_1[0x95] = &PTR_FUN_007ea4e4` | 0x95 | **+0x254** | vtable | SuperWeapons DVC vtable |
| `param_1[0x9a] = 10` | 0x9A | **+0x268** | 10 | grow_amount |
| `param_1[0x99] = 0` | 0x99 | **+0x264** | 0 | count |

---

## Phase 9: Chosen Type Indices and CellStruct Fields (+0x26C..+0x2B4)

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `param_1[0x9b] = 0xffffffff` | 0x9B | **+0x26C** | -1 | Unknown type index |
| `param_1[0x9c] = 0xffffffff` | 0x9C | **+0x270** | -1 | Unknown type index |
| `param_1[0x9d] = 0xffffffff` | 0x9D | **+0x274** | -1 | Unknown type index |
| `param_1[0x9e] = 0xffffffff` | 0x9E | **+0x278** | -1 | Unknown type index |
| `param_1[0x9f] = 0` | 0x9F | **+0x27C** | 0 | Unknown |
| `param_1[0xa0] = DAT_00a8ed84` | 0xA0 | **+0x280** | g_FrameCounter | Timer start frame |
| `param_1[0xa2] = 0` | 0xA2 | **+0x288** | 0 | Timer duration |
| `param_1[0xa5] = 0` | 0xA5 | **+0x294** | 0 | Timer duration |
| `param_1[0xa3] = DAT_00a8ed84` | 0xA3 | **+0x28C** | g_FrameCounter | Timer start frame |
| `param_1[0xa6] = DAT_00a8ed84` | 0xA6 | **+0x298** | g_FrameCounter | WinLoss timer start |
| `param_1[0xa8] = 0` | 0xA8 | **+0x2A0** | 0 | BorrowedTimeFrames |
| `param_1[0xab] = 0` | 0xAB | **+0x2AC** | 0 | Timer duration |
| `param_1[0xa9] = DAT_00a8ed84` | 0xA9 | **+0x2A4** | g_FrameCounter | Speech timer start |
| `param_1[0xac] = DAT_00a8ed84` | 0xAC | **+0x2B0** | g_FrameCounter | Announcement timer start |
| `param_1[0xae] = 0` | 0xAE | **+0x2B8** | 0 | Timer duration |

Timer pattern: `[start_frame, unused, duration]` where start=g_FrameCounter, duration=0.
These are CDTimerClass instances (12 bytes each).

---

## Phase 10: More Flags and Counters (+0x2BC..+0x2F8)

| Decompiled | Byte Offset | Value | Field |
|-----------|-------------|-------|-------|
| `*(byte*)(param_1 + 0xaf) = 0` | **+0x2BC** | 0 | Unknown byte |
| bytes at +0x2BD, +0x2BE, +0x2BF = 0 | **+0x2BD-2BF** | 0 | Padding/flags |
| `*(byte*)(param_1 + 0xb0) = 0` | **+0x2C0** | 0 | Unknown byte |
| `param_1[0xb1] = 0` | 0xB1 | **+0x2C4** | 0 | Unknown |
| `param_1[0xb2] = 0` | 0xB2 | **+0x2C8** | 0 | Unknown |
| `param_1[0xb3] = 0` | 0xB3 | **+0x2CC** | 0 | Unknown |
| `param_1[0xb4] = 0` | 0xB4 | **+0x2D0** | 0 | Unknown |
| `param_1[0xb5] = 0` | 0xB5 | **+0x2D4** | 0 | Unknown |
| `param_1[0xb6] = 0` | 0xB6 | **+0x2D8** | 0 | RobotControlCount |
| `param_1[0xb7] = 0` | 0xB7 | **+0x2DC** | 0 | TotalCreditsSpent |
| `param_1[0xb8] = 0` | 0xB8 | **+0x2E0** | 0 | Unknown |
| `param_1[0xb9] = 0` | 0xB9 | **+0x2E4** | 0 | Unknown |
| `param_1[0xba] = 0` | 0xBA | **+0x2E8** | 0 | InfantryCount |
| `param_1[0xbb] = 0` | 0xBB | **+0x2EC** | 0 | Unknown (counter object start?) |
| `param_1[0xbc] = 0` | 0xBC | **+0x2F0** | 0 | BuildingCount |
| `param_1[0xbd] = 0` | 0xBD | **+0x2F4** | 0 | AircraftCount |
| `param_1[0xbe] = 0` | 0xBE | **+0x2F8** | 0 | VehicleCount |

---

## Phase 11: Embedded Counter Objects (FUN_006c95e0)

```c
FUN_006c95e0();  // Counter object at +0x2FC (or nearby)
param_1[0xc3] = 0;  // +0x30C = AvailableCredits
param_1[0xc4] = 0;  // +0x310 = TrackedTiberiumBalance
FUN_006c95e0();  // Second counter object
```

FUN_006c95e0 initializes a counter/storage embedded struct. The two fields between them:

| Index | Byte Offset | Value | Field |
|-------|-------------|-------|-------|
| 0xC3 | **+0x30C** | 0 | AvailableCredits |
| 0xC4 | **+0x310** | 0 | TrackedTiberiumBalance |

---

## Phase 12: Timer Objects (10x FUN_00748fd0)

```c
FUN_00748fd0();  // ×10 — initializes 10 CDTimerClass or rate-tracking objects
```

These 10 calls initialize timer/rate-tracking embedded objects in a contiguous block.
Each FUN_00748fd0 likely initializes a small timer struct. The exact size per timer
depends on the struct, but they span a region from roughly +0x314 through the area
before +0x5378.

---

## Phase 13: Production Tracking Fields (+0x5378..+0x53D8)

After the 10 timer inits, the code jumps to high-offset fields. Since param_1 is
`undefined4*`, index 0x14DE = byte offset **0x5378**.

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `param_1[0x14de] = 0` | 0x14DE | **+0x5378** | 0 | QueuedProductionCount[0] |
| `param_1[0x14df] = 0` | 0x14DF | **+0x537C** | 0 | QueuedProductionCount[1] |
| `param_1[0x14e0] = 0` | 0x14E0 | **+0x5380** | 0 | QueuedProductionCount[2] |
| `param_1[0x14e1] = 0` | 0x14E1 | **+0x5384** | 0 | QueuedProductionCount[3] |
| `param_1[0x14e2] = 0` | 0x14E2 | **+0x5388** | 0 | QueuedProductionCount[4] |
| `param_1[0x14e3] = 0` | 0x14E3 | **+0x538C** | 0 | Unknown |

### Build Speed Bonuses (floats, 0x3F800000 = 1.0f)

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `param_1[0x14e4] = 0x3f800000` | 0x14E4 | **+0x5390** | **1.0f** | BuildSpeedBonus_Infantry |
| `param_1[0x14e5] = 0x3f800000` | 0x14E5 | **+0x5394** | **1.0f** | BuildSpeedBonus_Naval |
| `param_1[0x14e6] = 0x3f800000` | 0x14E6 | **+0x5398** | **1.0f** | BuildSpeedBonus_Air |
| `param_1[0x14e7] = 0x3f800000` | 0x14E7 | **+0x539C** | **1.0f** | BuildSpeedBonus_Vehicle |
| `param_1[0x14e8] = 0x3f800000` | 0x14E8 | **+0x53A0** | **1.0f** | BuildSpeedBonus_VehicleAlt |

### Attack/Defense Power and Factory Pointers

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `param_1[0x14e9] = 0` | 0x14E9 | **+0x53A4** | 0 | AttackPowerSum |
| `param_1[0x14ea] = 0` | 0x14EA | **+0x53A8** | 0 | DefensePowerSum |
| `param_1[0x14eb] = 0` | 0x14EB | **+0x53AC** | 0 | InfantryFactory* |
| `param_1[0x14ec] = 0` | 0x14EC | **+0x53B0** | 0 | AircraftFactory* |
| `param_1[0x14ed] = 0` | 0x14ED | **+0x53B4** | 0 | UnitFactory* |
| `param_1[0x14ee] = 0` | 0x14EE | **+0x53B8** | 0 | NavalFactory* |
| `param_1[0x14ef] = 0` | 0x14EF | **+0x53BC** | 0 | BuildingFactory* |
| `param_1[0x14f0] = 0` | 0x14F0 | **+0x53C0** | 0 | Unknown factory |
| `param_1[0x14f1] = 0` | 0x14F1 | **+0x53C4** | 0 | Unknown factory |
| `param_1[0x14f2] = 0` | 0x14F2 | **+0x53C8** | 0 | BuildingFactoryAlt* |
| `param_1[0x14f3] = 0` | 0x14F3 | **+0x53CC** | 0 | BuildingFactoryAlt2* |

### Byte Flags at +0x53D0..+0x53D8

| Decompiled | Byte Offset | Value | Field |
|-----------|-------------|-------|-------|
| `*(byte*)(param_1 + 0x14f4) = 0` | **+0x53D0** | 0 | Unknown |
| bytes at +0x53D1..+0x53D3 = 0 | **+0x53D1-D3** | 0 | |
| `*(byte*)(param_1 + 0x14f5) = 0` | **+0x53D4** | 0 | Unknown |
| bytes at +0x53D5..+0x53D7 = 0 | **+0x53D5-D7** | 0 | |
| `*(byte*)(param_1 + 0x14f6) = 0` | **+0x53D8** | 0 | Unknown |

---

## Phase 14: Rally Point Fields (+0x53DC..+0x53E4)

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `param_1[0x14f7] = 0` | 0x14F7 | **+0x53DC** | 0 | RallyPointObject |
| `*(short*)(param_1 + 0x14f8) = 0` | **+0x53E0** | 0 | RallyPointCell.X |
| `*(short*)((int)param_1 + 0x53e2) = 0` | **+0x53E2** | 0 | RallyPointCell.Y |

---

## Phase 15: Known-Object Count Arrays (+0x5434..+0x548C)

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `param_1[0x150d] = 0` | 0x150D | **+0x5434** | 0 | Unknown (start of array region) |

---

## Phase 16: AI Strategy Fields (+0x5488..+0x54FC)

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `param_1[0x1522] = 0` | 0x1522 | **+0x5488** | 0 | Unknown |
| `param_1[0x1523] = 0xffffffff` | 0x1523 | **+0x548C** | -1 | Unknown index (-1 sentinel) |
| `param_1[0x1524] = DAT_00a8ef98` | 0x1524 | **+0x5490** | InvalidCell | BaseCenterCell |
| `param_1[0x1525] = DAT_00a8ef98` | 0x1525 | **+0x5494** | InvalidCell | AltBaseCenterCell |
| `param_1[0x1526] = 0` | 0x1526 | **+0x5498** | 0 | BaseSpreadRadius |
| `param_1[0x1536] = 0` | 0x1536 | **+0x54D8** | 0 | BuildCooldownTimer |
| `param_1[0x1537] = 0xffffffff` | 0x1537 | **+0x54DC** | -1 | Unknown index |
| `param_1[0x1538] = 0` | 0x1538 | **+0x54E0** | 0 | Unknown |
| `param_1[0x1539] = 0` | 0x1539 | **+0x54E4** | 0 | Unknown |
| `param_1[0x153a] = 0` | 0x153A | **+0x54E8** | 0 | LastDepositAmount |
| `param_1[0x153b] = 1` | 0x153B | **+0x54EC** | **1** | Unknown (default true) |

### Rally Cell Shorts

| Decompiled | Byte Offset | Value | Field |
|-----------|-------------|-------|-------|
| `*(short*)(param_1 + 0x153c) = 0` | **+0x54F0** | 0 | PrimaryRallyCell.X |
| `*(short*)((int)param_1 + 0x54f2) = 0` | **+0x54F2** | 0 | PrimaryRallyCell.Y |
| `*(short*)(param_1 + 0x153d) = 0` | **+0x54F4** | 0 | SecondaryRallyCell.X |
| `*(short*)((int)param_1 + 0x54f6) = 0` | **+0x54F6** | 0 | SecondaryRallyCell.Y |
| `*(short*)(param_1 + 0x153e) = 0` | **+0x54F8** | 0 | Unknown short pair |
| `*(short*)((int)param_1 + 0x54fa) = 0` | **+0x54FA** | 0 | Unknown short pair |
| `param_1[0x153f] = 0xffffff9c` | 0x153F | **+0x54FC** | **-100** | SecondaryRallyFrame (expired sentinel) |

---

## Phase 17: IndexClass Arrays (12x FUN_0049f9b0)

```c
FUN_0049f9b0();  // ×12
```

Creates 12 IndexClass arrays for tracking owned unit counts by type (building, infantry,
unit, aircraft, etc.). Each IndexClass is 20 bytes (0x14):

```c
struct IndexClass {  // 20 bytes
    void* vtable;     // +0x00 = PTR_FUN_007e5c54
    int*  data;       // +0x04 = pointer to int array
    int   capacity;   // +0x08
    byte  can_grow;   // +0x0C
    byte  pad[3];
    int   total;      // +0x10 = total count across all types
};
```

12 arrays x 20 bytes = 240 bytes total, starting from the constructor's current position.
These track quantities of: BuildingTypes owned, InfantryTypes owned, UnitTypes owned,
AircraftTypes owned, BuildingTypes killed, InfantryTypes killed, UnitTypes killed,
AircraftTypes killed, BuildingTypes captured, InfantryTypes captured, UnitTypes captured,
AircraftTypes captured.

---

## Phase 18: Cell/Location Fields (+0x55F0..+0x5600)

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `param_1[0x157c] = DAT_00a8ed84` | 0x157C | **+0x55F0** | g_FrameCounter | Timer start |
| `param_1[0x157e] = 0` | 0x157E | **+0x55F8** | 0 | Unknown |
| `param_1[0x157f] = 0` | 0x157F | **+0x55FC** | 0 | Unknown |
| `param_1[0x1580] = 0xffffffff` | 0x1580 | **+0x5600** | -1 | EnemyHouseIndex |

---

## Phase 19: GrudgeList DVC (FUN_005106e0) at +0x5604

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `FUN_005106e0(0,0)` | -- | -- | -- | Constructs DVC at +0x5604 |
| `param_1[0x1581] = &PTR_FUN_007ea924` | 0x1581 | **+0x5604** | vtable | GrudgeList DVC |
| `param_1[0x1586] = 10` | 0x1586 | **+0x5618** | 10 | grow_amount |
| `param_1[0x1585] = 0` | 0x1585 | **+0x5614** | 0 | count |

This is a DVC of 8-byte entries: `[HouseClass*, int score]`.

---

## Phase 20: ThreatSourceList DVC (FUN_00510780) at +0x561C

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `FUN_00510780(0,0)` | -- | -- | -- | Constructs DVC at +0x561C |
| `param_1[0x1587] = &PTR_FUN_007ea904` | 0x1587 | **+0x561C** | vtable | ThreatSource DVC |
| `param_1[0x158c] = 10` | 0x158C | **+0x5630** | 10 | grow_amount |
| `param_1[0x158b] = 0` | 0x158B | **+0x562C** | 0 | count |

---

## Phase 21: AI Strategy Cell Fields (+0x5634..+0x566C)

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `param_1[0x158d] = DAT_00a8ed84` | 0x158D | **+0x5634** | g_FrameCounter | AI strategy timer start |
| `param_1[0x158f] = 0` | 0x158F | **+0x563C** | 0 | AI strategy timer duration |
| `param_1[0x1590] = DAT_00a8ed84` | 0x1590 | **+0x5640** | g_FrameCounter | Timer start |
| `param_1[0x1592] = 0` | 0x1592 | **+0x5648** | 0 | Timer duration |
| `param_1[0x1593] = 0xffffffff` | 0x1593 | **+0x564C** | -1 | ChosenBuildingType |
| `param_1[0x1594] = 0xffffffff` | 0x1594 | **+0x5650** | -1 | ChosenUnitType |
| `param_1[0x1595] = 0xffffffff` | 0x1595 | **+0x5654** | -1 | ChosenAircraftType |
| `param_1[0x1596] = 0xffffffff` | 0x1596 | **+0x5658** | -1 | ChosenInfantryType |
| `param_1[0x1597] = 100` | 0x1597 | **+0x565C** | **100** | RatioAITriggerTeam |
| `param_1[0x1598] = 0x4b` | 0x1598 | **+0x5660** | **75** | RatioTeamAircraft |
| `param_1[0x1599] = 0x4b` | 0x1599 | **+0x5664** | **75** | RatioTeamInfantry |
| `param_1[0x159a] = 0x4b` | 0x159A | **+0x5668** | **75** | RatioTeamUnits |
| `param_1[0x159b] = 0` | 0x159B | **+0x566C** | 0 | Unknown |

---

## Phase 22: Build Multiplier Arrays (3x FUN_004b69b0)

```c
local_34 = 3;
do {
    FUN_004b69b0();
    local_34 = local_34 + -1;
} while (local_34 != 0);
```

3 calls to FUN_004b69b0, which initializes build-speed/cost multiplier array objects.
These likely correspond to three object-category multiplier groups.

---

## Phase 23: Base Plan and Color Fields (+0x56F4..+0x56FE)

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `param_1[0x15bd] = 0` | 0x15BD | **+0x56F4** | 0 | Unknown |
| `*(byte*)(param_1 + 0x15be) = 0` | **+0x56F8** | 0 | Unknown byte |
| bytes at +0x56F9..+0x56FB = 0 | **+0x56F9-FB** | 0 | HouseColorRGB (black initially) |

### Color Fields

| Decompiled | Byte Offset | Value | Field |
|-----------|-------------|-------|-------|
| `*(byte*)(param_1 + 0x15bf) = 0xff` | **+0x56FC** | **0xFF** | HouseBrightR |
| `*((int)param_1 + 0x56fd) = 0xff` | **+0x56FD** | **0xFF** | HouseBrightG |
| `*((int)param_1 + 0x56fe) = 0xff` | **+0x56FE** | **0xFF** | HouseBrightB |

Color default: RGB = (0,0,0) black, Bright remap = (255,255,255) white.

---

## Phase 24: FUN_0042e6f0 Call

```c
FUN_0042e6f0();
```

Initializes some combat-related embedded state (exact nature unknown, possibly
WaypointPathClass or similar).

---

## Phase 25: Speech/Announcement Flags (+0x5778..+0x578C)

| Decompiled | Byte Offset | Value | Field |
|-----------|-------------|-------|-------|
| `*(byte*)(param_1 + 0x15de) = 1` | **+0x5778** | **1** | SpeechPending |
| `*((int)param_1 + 0x5779) = 1` | **+0x5779** | **1** | AnnouncementPending |
| `*((int)param_1 + 0x577a) = 0` | **+0x577A** | 0 | LowPowerState |
| `*((int)param_1 + 0x577b) = 0` | **+0x577B** | 0 | HasOffensiveUnits |
| `param_1[0x15df] = 0` | 0x15DF | **+0x577C** | 0 | EdgeDirection |
| `*(short*)(param_1 + 0x15e0) = 0` | **+0x5780** | 0 | Unknown short |
| `*(short*)((int)param_1 + 0x5782) = 0` | **+0x5782** | 0 | Unknown short |
| `*(short*)(param_1 + 0x15e1) = 0` | **+0x5784** | 0 | Unknown short |
| `*(short*)((int)param_1 + 0x5786) = 0` | **+0x5786** | 0 | Unknown short |
| `param_1[0x15e2] = 0` | 0x15E2 | **+0x5788** | 0 | AllianceBitfield (no allies) |

---

## Phase 26: Timer Fields at +0x5794..+0x57AC

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `uVar4 = FUN_007c5f00()` | -- | -- | -- | timeGetTime() |
| `param_1[0x15e5] = uVar4` | 0x15E5 | **+0x5794** | timeGetTime | Unknown timestamp |
| `param_1[0x15e3] = DAT_00a8ed84` | 0x15E3 | **+0x578C** | g_FrameCounter | Unknown timer start |
| `param_1[0x15e6] = DAT_00a8ed84` | 0x15E6 | **+0x5798** | g_FrameCounter | AI trigger timer start |
| `param_1[0x15e8] = 1` | 0x15E8 | **+0x57A0** | **1** | AI trigger timer duration |
| `param_1[0x15e9] = DAT_00a8ed84` | 0x15E9 | **+0x57A4** | g_FrameCounter | AnnounceReady timer start |
| `param_1[0x15eb] = 0` | 0x15EB | **+0x57AC** | 0 | Timer duration |
| `param_1[0x15ec] = DAT_00a8ed84` | 0x15EC | **+0x57B0** | g_FrameCounter | Timer start |
| `param_1[0x15ee] = 1` | 0x15EE | **+0x57B8** | **1** | Timer duration |
| `param_1[0x15ef] = DAT_00a8ed84` | 0x15EF | **+0x57BC** | g_FrameCounter | LowPowerWarning timer start |
| `param_1[0x15f1] = 1` | 0x15F1 | **+0x57C4** | **1** | Timer duration |
| `param_1[0x15f2] = DAT_00a8ed84` | 0x15F2 | **+0x57C8** | g_FrameCounter | Timer start |
| `param_1[0x15f4] = 1` | 0x15F4 | **+0x57D0** | **1** | Timer duration |
| `param_1[0x15f5] = DAT_00a8ed84` | 0x15F5 | **+0x57D4** | g_FrameCounter | MoneyWarning timer start |
| `param_1[0x15f7] = 1` | 0x15F7 | **+0x57DC** | **1** | Timer duration |
| `param_1[0x15f8] = 0` | 0x15F8 | **+0x57E0** | 0 | Unknown |

---

## Phase 27: Threat Map and Player Name (+0x57F4..+0x1602A)

| Decompiled | Byte Offset | Value | Field |
|-----------|-------------|-------|-------|
| `*(byte*)(param_1 + 0x57fd) = 0` | **+0x15FF4** | 0 | PlayerName[0] (empty string) |
| `*((int)param_1 + 0x16009) = 0` | **+0x16009** | 0 | UIName[0] (empty string) |

**WAIT -- important offset check.** `param_1 + 0x57fd` in pointer arithmetic means
`(int)param_1 + 0x57fd*4 = (int)param_1 + 0x15FF4`. And `(int)param_1 + 0x16009` is
a direct byte offset. So:

- PlayerName starts at byte offset **+0x15FF4** (from `param_1 + 0x57FD`, dword index)
- UIName starts at byte offset **+0x16009** (direct cast)

---

## Phase 28: High-Offset Fields (+0x16054..+0x160B4)

These use very large indices because param_1 is `undefined4*`:

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `param_1[0x5815] = 0` | 0x5815 | **+0x16054** | 0 | ColorSchemeIndex |
| `param_1[0x5818] = 0` | 0x5818 | **+0x16060** | 0 | SideFlags/DefaultFlags |

### DVC at +0x16064 (FUN_005105f0) -- Shroud/Visibility

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `FUN_005105f0(0,0)` | -- | -- | -- | Constructs DVC at +0x16064 |
| `param_1[0x5819] = &PTR_FUN_007e5d48` | 0x5819 | **+0x16064** | vtable | Unknown DVC |
| `param_1[0x581e] = 10` | 0x581E | **+0x16078** | 10 | grow_amount |
| `param_1[0x581d] = 0` | 0x581D | **+0x16074** | 0 | count |

### More High-Offset Fields

| Decompiled | Index | Byte Offset | Value | Field |
|-----------|-------|-------------|-------|-------|
| `param_1[0x581f] = 0` | 0x581F | **+0x1607C** | 0 | CellSpreadClass* |
| `param_1[0x5820] = 0` | 0x5820 | **+0x16080** | 0 | Unknown |
| `param_1[0x5822] = 0` | 0x5822 | **+0x16088** | 0 | Unknown |

### Doubles at high offsets

| Index | Byte Offset | Value (hex) | Value (float) | Field |
|-------|-------------|-------------|---------------|-------|
| 0x5823 | **+0x1608C** | 0x3FF00000 | ~1.0 (double high) | Unknown double |
| 0x5824 | **+0x16090** | 0 | | Unknown double low |
| 0x5825 | **+0x16094** | 0x3FF00000 | ~1.0 (double high) | Unknown double |

**Wait -- correction.** These are stored as individual dwords, not double pairs. Looking
more carefully: `0x3FF00000` as a standalone float would be garbage. But indices 0x5823
and 0x5824 together could form a double, with 0x5822=0 (low) and 0x5823=0x3FF00000 (high)
= 1.0 double. Let me re-examine:

Actually, re-reading the code:
```c
param_1[0x5822] = 0;          // +0x16088 (low dword)
param_1[0x5823] = 0x3ff00000; // +0x1608C (high dword) → double 1.0
param_1[0x5824] = 0;          // +0x16090 (low dword)
param_1[0x5825] = 0x3ff00000; // +0x16094 (high dword) → double 1.0
```

So these are 2 doubles initialized to 1.0.

### AI Ratio Floats (0x3EA8F5C3 = 0.33f)

| Index | Byte Offset | Value (hex) | Value (float) | Field |
|-------|-------------|-------------|---------------|-------|
| 0x5827 | **+0x1609C** | 0x3EA8F5C3 | **~0.33f** | AIInfantryRatio |
| 0x5828 | **+0x160A0** | 0x3EA8F5C3 | **~0.33f** | AIVehicleRatio |
| 0x5829 | **+0x160A4** | 0x3EA8F5C3 | **~0.33f** | AIAircraftRatio |

### Final Zero Fields

| Index | Byte Offset | Value | Field |
|-------|-------------|-------|-------|
| 0x582A | **+0x160A8** | 0 | TrackedAircraftValue |
| 0x582B | **+0x160AC** | 0 | TrackedInfantryValue |
| 0x582C | **+0x160B0** | 0 | TrackedGeneralValue |
| 0x582D | **+0x160B4** | 0 | AICostTolerance |

Object ends at +0x160B8 (total size confirmed).

---

## Phase 29: Vtable Pointer Assignments (Multiple Inheritance)

After all field initialization, the vtable pointers are written to the very beginning
of the object. HouseClass uses multiple inheritance with at least 4 COM-style interfaces:

```c
*param_1     = &PTR_FUN_007ea8a0;    // +0x00: Primary HouseClass vtable
param_1[1]   = &PTR_LAB_007ea884;    // +0x04: IPublicHouse vtable
param_1[2]   = &PTR_LAB_007ea87c;    // +0x08: IHouse vtable
param_1[3]   = &PTR_LAB_007ea874;    // +0x0C: IOther vtable
param_1[9]   = &PTR_LAB_007ea834;    // +0x24: Unknown interface vtable
param_1[10]  = &PTR_LAB_007ea80c;    // +0x28: Unknown interface vtable
param_1[0xb] = &PTR_LAB_007ea7f4;    // +0x2C: Unknown interface vtable
```

Note: These are set near the END of the constructor, not the beginning. This is typical
C++ behavior -- vtables are written after base class constructors complete, to ensure
virtual dispatch resolves to the most-derived class.

The 7 vtable pointers occupy offsets +0x00, +0x04, +0x08, +0x0C, +0x24, +0x28, +0x2C.
The gaps +0x10..+0x20 are AbstractClass fields (UniqueID, RTTI, etc.).

---

## Phase 30: UIName Copy from CountryType

```c
_strncpy(local_20, &DAT_00889f64, 0x1f);  // Copy default UIName (31 chars)
local_1 = 0;                                // Null terminator
// memcpy to param_1 + 0x16009 (32 bytes)
if (local_20 != (char*)((int)param_1 + 0x16009)) {
    for (iVar8 = 8; iVar8 != 0; iVar8--) {
        *puVar7 = *(undefined4*)pcVar9;  // Copy 8 dwords = 32 bytes
    }
}
```

DAT_00889f64 is likely a default UIName string constant.
Destination: byte offset **+0x16009** (UIName, 32 bytes).

---

## Phase 31: AbstractClass Registration

```c
*(undefined2*)((int)param_1 + 0x1602a) = 0;  // +0x1602A: zero (short)
FUN_00410230(param_1 + 1);                    // Register in AbstractClass global array
```

FUN_00410230 sets up the networking/tracking ID in the base class layer.
`param_1 + 1` in pointer arithmetic = byte offset +0x04 = the IPublicHouse vtable slot,
which is the start of the first "base class" subobject.

---

## Phase 32: Copy SideFlags from CountryType

```c
if (param_1[0xd] != 0) {  // if HouseTypeClass* != NULL
    param_1[0x5815] = *(undefined4*)(param_1[0xd] + 0xc0);
    // +0x16054 = CountryType->Color (offset +0xC0 in CountryTypeClass)
}
```

Copies the color scheme index from the CountryTypeClass to HouseClass+0x16054.

---

## Phase 33: Registration in 5 Global DynamicVector Arrays

The house registers itself in 5 separate global arrays. Each uses the same pattern:
check capacity, grow if needed, then append `this` pointer.

```c
// Array 1: DAT_00b0f674 (data ptr), DAT_00b0f680 (count), DAT_00b0f678 (capacity)
DAT_00b0f674[DAT_00b0f680++] = param_1;

// Array 2: DAT_00b0f644 (data ptr), DAT_00b0f650 (count), DAT_00b0f648 (capacity)
DAT_00b0f644[DAT_00b0f650++] = param_1;

// Array 3: DAT_00b0f5f4 (data ptr), DAT_00b0f600 (count), DAT_00b0f5f8 (capacity)
DAT_00b0f5f4[DAT_00b0f600++] = param_1;

// Array 4: DAT_00b0f61c (data ptr), DAT_00b0f628 (count), DAT_00b0f620 (capacity)
DAT_00b0f61c[DAT_00b0f628++] = param_1;

// Array 5: DAT_00b0f724 (data ptr), DAT_00b0f730 (count), DAT_00b0f728 (capacity)
DAT_00b0f724[DAT_00b0f730++] = param_1;
```

These 5 arrays represent different inheritance hierarchy registrations (AbstractClass,
TechnoClass, ObjectClass, etc.).

---

## Phase 34: Set HouseIndex from Master Array

```c
param_1[0xc] = DAT_00a80238;  // +0x30 = HouseIndex = current count (becomes this house's index)
```

Then the house is added to the master HouseClass::Array:
```c
DAT_00a8022c[DAT_00a80238++] = param_1;  // Append to master array, increment count
```

---

## Phase 35: Cross-Registration Loop with Existing Houses

This is the big loop at the end. For each existing house (`local_34 = 0..DAT_00a80238-1`):

```c
if (param_2 != 0) {  // Only if CountryType is provided
    for (local_34 = 0; local_34 < DAT_00a80238; local_34++) {
        int otherHouse = DAT_00a8022c[local_34];

        // 1. Add THIS to otherHouse's GrudgeList (+0x5604)
        //    8-byte entry: [this_ptr, 0]
        otherHouse->GrudgeList[count++] = {param_1, 0};

        // 2. Add otherHouse to THIS house's AltHouseList (+0x5608)
        //    (wait -- this uses param_1[0x1582..0x1585], which is +0x5608..+0x5614)
        this->GrudgeListMirror[count++] = {otherHouse, 0};

        // 3. Add THIS to otherHouse's ThreatSourceList (+0x561C)
        //    8-byte entry: [this_ptr, flag]
        otherHouse->ThreatSourceList[count++] = {param_1, flag};

        // 4. Add otherHouse to THIS house's ThreatSourceMirror (+0x561C)
        this->ThreatSourceListMirror[count++] = {otherHouse, flag};
    }
}
```

This establishes mutual diplomacy tracking between the new house and all existing houses.
The 8-byte entries store `[HouseClass* pointer, int score/flag]`.

---

## Phase 36: Zero Threat Map Grid

```c
puVar7 = param_1 + 0x15f9;  // Byte offset: 0x15F9 * 4 = +0x57E4
for (iVar8 = 0x4204; iVar8 != 0; iVar8--) {
    *puVar7 = 0;
    puVar7 = puVar7 + 1;
}
```

Zeros **0x4204 dwords** (16,912 dwords = 67,648 bytes) starting at offset **+0x57E4**.
This is the ThreatMapGrid -- a large per-cell threat value array (~130x130 cells).

---

## Phase 37: SuperWeapon Array Creation Loop

```c
local_34 = 0;
if (0 < DAT_00a8e340) {  // DAT_00a8e340 = SuperWeaponTypeClass::Array.Count
    do {
        pvVar5 = operator_new(0x80);  // Allocate 128 bytes for SuperClass
        if (pvVar5 == NULL) {
            uVar6 = 0;
        } else {
            // FUN_006caf90 = SuperClass::SuperClass(SuperWeaponTypeClass*, HouseClass*)
            uVar6 = FUN_006caf90(
                DAT_00a8e334[local_34],  // SuperWeaponTypeClass* from global array
                param_1                   // owner = this HouseClass
            );
        }
        // Add to SuperWeapons DVC at +0x254
        // param_1[0x97] = +0x25C = DVC.capacity
        // param_1[0x99] = +0x264 = DVC.count
        // param_1[0x96] = +0x258 = DVC.data
        param_1[0x99]++;
        param_1[0x96][count * 4] = uVar6;  // Store SuperClass*

        local_34++;
    } while (local_34 < DAT_00a8e340);
}
```

**Key details:**
- `DAT_00a8e340` = global count of SuperWeaponTypeClass instances
- `DAT_00a8e334` = pointer to SuperWeaponTypeClass*[] array
- Each SuperClass is **0x80 bytes** (128 bytes)
- Constructed by FUN_006caf90 with `(type_ptr, owner_house)`
- Stored in the DVC at HouseClass+0x254

---

## Phase 38: Zero Known-Object Arrays

```c
// Zero 0x14 (20) dwords at param_1 + 0x14f9 = +0x53E4
puVar7 = param_1 + 0x14f9;
for (iVar8 = 0x14; iVar8 != 0; iVar8--) { *puVar7++ = 0; }

// Zero 0x14 (20) dwords at param_1 + 0x150e = +0x5438
puVar7 = param_1 + 0x150e;
for (iVar8 = 0x14; iVar8 != 0; iVar8--) { *puVar7++ = 0; }
```

Two arrays of 20 dwords each:
- **+0x53E4..+0x5434** (80 bytes) -- likely BuildingsOwned count array
- **+0x5438..+0x548C** (80 bytes) -- likely BuildingsKilled count array

---

## Phase 39: Copy Player Name from CountryType

```c
if (param_2 != 0) {
    if (param_2 + 0x24 == NULL) {
        local_20[0] = '\0';
    } else {
        _strncpy(local_20, (char*)(param_2 + 0x24), 0x14);  // 20 chars from CountryType+0x24
        uStack_c = 0;
    }
    // Copy 20+1 bytes to param_1 + 0x57fd = +0x15FF4
    if (local_20 != (char*)(param_1 + 0x57fd)) {
        for (iVar8 = 5; iVar8 != 0; iVar8--) {
            // Copy 5 dwords = 20 bytes
        }
        *pcVar10 = *pcVar9;  // Copy 1 more byte (null terminator)
    }
    // Copy UIName CSF string
    FUN_007ca489((int)param_1 + 0x1602a, *(undefined4*)(param_2 + 0x60));
}
```

- **PlayerName** at +0x15FF4: 20 chars from CountryTypeClass+0x24 (section name)
- **UINameCSF** at +0x1602A: string from CountryTypeClass+0x60

---

## Phase 40: Zero Threat Map Again + Set Alliance Bit

```c
// Zero threat map again (same 0x4204 dwords at +0x57E4)
puVar7 = param_1 + 0x15f9;
for (iVar8 = 0x4204; iVar8 != 0; iVar8--) { *puVar7++ = 0; }

// Set self-alliance bit
param_1[0x76] = param_1[0x76] | (1 << ((byte)param_1[0xc] & 0x1f));
// +0x1D8 (RadarShareBitfield) |= (1 << HouseIndex)
// Actually +0x1D8 is the alliance bitfield. A house is always allied with itself.
```

**Correction on the field name:** `param_1[0x76]` = byte offset +0x1D8 = this was labeled
RadarShareBitfield above, but this line sets the self-alliance bit. Cross-referencing with
the existing report:
- +0x1D8 = RadarShareBitfield (per existing docs)
- +0x5788 = AllianceBitfield (per existing docs)

However, this code at `param_1[0x76]` = +0x1D8 is setting a "self" bit, and from IsAlliedWith
at 0x4f9a50 which reads +0x5788... Let me reconsider. The alliance self-bit is likely at +0x1D8,
used for a different purpose (possibly a "houses known" or "active houses" bitmask). The
alliance bitfield at +0x5788 is the one checked by IsAlliedWith.

---

## Phase 41: Event Notifications (FUN_0065c7e0)

```c
FUN_0065c7e0(0x1c2, 0x708);
```

Sends a network/event notification. Parameters might be event type and data.

---

## Phase 42: Re-initialize Timers

```c
uVar6 = FUN_007c5f00();          // timeGetTime()
param_1[0x157c] = DAT_00a8ed84;  // +0x55F0 = g_FrameCounter
param_1[0x157d] = uStack_1c;     // +0x55F4 = unknown
param_1[0x157e] = uVar6;         // +0x55F8 = timeGetTime result
param_1[0x157f] = uVar6;         // +0x55FC = timeGetTime result (copy)
```

---

## Phase 43: FUN_00749060 Calls (10x)

```c
FUN_00749060(DAT_00a8b228);  // 4 different global params, called 10 times total
FUN_00749060(DAT_00a8e358);
FUN_00749060(DAT_00a83cf0);
FUN_00749060(DAT_00a83c78);
FUN_00749060(DAT_00a8b228);
FUN_00749060(DAT_00a8e358);
FUN_00749060(DAT_00a83cf0);
FUN_00749060(DAT_00a83c78);
FUN_00749060(DAT_00a83c78);
FUN_00749060(0x13);
```

These are likely type-array registrations or notifications. The global addresses:
- DAT_00a8b228 = InfantryTypeClass::Array
- DAT_00a8e358 = AircraftTypeClass::Array
- DAT_00a83cf0 = UnitTypeClass::Array
- DAT_00a83c78 = BuildingTypeClass::Array

---

## Phase 44: Zero Production Fields

```c
puVar7 = param_1 + 0x84;  // +0x210
for (iVar8 = 0xc; iVar8 != 0; iVar8--) { *puVar7++ = 0; }
```

Zeros 12 dwords at +0x210 = **FactorySlots** array (12 production slots, all NULL).

---

## Phase 45: Set NeedsRebuild Flag

```c
if (param_2 != 0) {
    *((byte*)((int)param_1 + 0x1fb)) = 1;  // +0x1FB = NeedsRebuild = true
}
```

Only set when CountryType is provided (not for neutral houses).

---

## Phase 46: Set SideIndex from CountryType

```c
param_1[0x7a] = 0xffffffff;  // +0x1E8 = SideIndex = -1 (default)
if (param_1[0xd] != 0) {     // if HouseTypeClass* != NULL
    iVar8 = *(int*)(param_1[0xd] + 0xbc);  // CountryType+0xBC = ParentCountryIndex
    if (iVar8 == 0) param_1[0x7a] = 0;      // Allied
    else if (iVar8 == 1) param_1[0x7a] = 1;  // Soviet
    else if (iVar8 == 2) param_1[0x7a] = 2;  // Yuri
}
```

SideIndex is derived from CountryTypeClass+0xBC (ParentCountryIndex), not a direct "Side"
field. Values: 0=Allied, 1=Soviet, 2=Yuri. -1 for houses without a CountryType.

---

## Phase 47: CellSpreadClass Creation

```c
FUN_004f6830(param_1, &DAT_007f7c90, &uStack_30);  // Some lookup/init
pvVar5 = operator_new(0x34);  // Allocate 52 bytes
if (pvVar5 != NULL) {
    puVar7 = FUN_004a0870(&DAT_007e9550, uStack_30);  // CellSpreadClass constructor
}
param_1[0x581f] = puVar7;  // +0x1607C = CellSpreadClass*
```

Allocates and stores a CellSpreadClass object (52 bytes) at +0x1607C.

---

## Phase 48: Final DVC Add + Return

```c
// Add something to the DVC at +0x16064
param_1[0x581d]++;  // count++
param_1[0x581a][count * 4] = unaff_EBX;  // Store value

return param_1;
```

---

## Summary: Complete Initialization Order

1. AbstractClass base constructor
2. HouseIndex = -1, HouseTypeClass* stored
3. 13 DynamicVectorClass arrays (capacity 10 each)
4. SpySat/Cloak/Power counters = 0
5. DifficultyLevel from global, 7 difficulty doubles = 1.0
6. RepairDelay/BuildDelay doubles = 0.0
7. IQ=0, TechLevel=1, RadarShare=0, Credits=0, StartingEdge=-1
8. 15 boolean flags at +0x1EC..+0x1FC (mostly 0, two set to 1)
9. CurrentFactoryIndex = -1
10. More byte flags at +0x240..+0x24B (SidebarUpdatePending=1)
11. CurrentIQ = IQ (0), SuperWeapons DVC
12. 4 chosen-type indices = -1
13. 6 CDTimerClass instances initialized with g_FrameCounter
14. Counter objects and tracking counters = 0
15. AvailableCredits=0, TrackedTiberium=0
16. 10 rate-tracking timer objects
17. 5 QueuedProductionCounts = 0
18. 5 BuildSpeedBonuses = 1.0f
19. Attack/Defense power = 0, 10 factory pointers = 0
20. Rally points = 0, SecondaryRallyFrame = -100
21. 12 IndexClass arrays for owned/killed type tracking
22. EnemyHouseIndex = -1
23. GrudgeList and ThreatSourceList DVCs
24. AI strategy timers, ChosenTypes = -1
25. AI ratios: TriggerTeam=100, Aircraft/Infantry/Units=75
26. 3 build multiplier array objects
27. Color RGB=(0,0,0), BrightRGB=(255,255,255)
28. Combat state init
29. SpeechPending=1, AnnouncementPending=1
30. AllianceBitfield=0
31. Multiple timers with g_FrameCounter start, various durations
32. PlayerName and UIName empty
33. ColorSchemeIndex=0, SideFlags=0
34. DVC at +0x16064
35. CellSpreadClass* = NULL initially
36. 2 doubles = 1.0, AI ratios = 0.33f
37. 4 tracking values = 0
38. **7 vtable pointers** written (multiple inheritance)
39. UIName copied from default constant
40. Register in AbstractClass array
41. Copy ColorScheme from CountryType
42. Register in 5 global arrays
43. Set HouseIndex from master array count
44. Add to master HouseClass::Array
45. Cross-register with all existing houses (diplomacy DVCs)
46. Zero 0x4204-dword threat map
47. Create SuperClass instances for each SuperWeaponType
48. Zero two 20-dword known-object arrays
49. Copy player name and UIName CSF from CountryType
50. Zero threat map again
51. Set self-alliance bit in bitfield
52. Event notification
53. Re-init timers with fresh timestamps
54. Type-array registrations (10x)
55. Zero 12 FactorySlots
56. Set NeedsRebuild if CountryType provided
57. Determine SideIndex from CountryType parent
58. Create CellSpreadClass

---

## Key Default Values Summary

| Value | Meaning | Fields Using It |
|-------|---------|-----------------|
| 0 | Zero/NULL/false | Most counters, pointers, credits, power |
| -1 (0xFFFFFFFF) | Invalid/unset sentinel | HouseIndex, ChosenTypes, EnemyHouse, CurrentFactory, StartingEdge, SideIndex |
| -100 (0xFFFFFF9C) | Expired timer sentinel | SecondaryRallyFrame |
| 1 | True/default | TechLevel, ProductionChanged, SidebarUpdatePending, +0x1F0 flag, SpeechPending, AnnouncementPending, various timer durations, NeedsRebuild |
| 1.0 (double, 0x3FF0000000000000) | Neutral multiplier | 7 difficulty doubles |
| 1.0f (float, 0x3F800000) | Neutral multiplier | 5 build speed bonuses |
| 0.33f (float, 0x3EA8F5C3) | AI balance ratio | AIInfantry/Vehicle/AircraftRatio |
| 10 | DVC grow amount | All DynamicVectorClass instances |
| 75 (0x4B) | Percentage | RatioTeamAircraft/Infantry/Units |
| 100 | Percentage | RatioAITriggerTeam |
| 255 (0xFF) | White channel | HouseBrightRGB |
| g_FrameCounter | Current tick | All CDTimerClass start frames |
| InvalidCell | No cell | BaseCenterCell, AltBaseCenterCell |
