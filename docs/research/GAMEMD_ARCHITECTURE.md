# gamemd.exe Complete Architecture Map

Reverse-engineered via Ghidra MCP (live decompilation of Yuri's Revenge `gamemd.exe`),
cross-referenced with RTTI type info, debug path strings, and known YRpp class layouts.

---

## Binary Overview

| Property | Value |
|---|---|
| File | `gamemd.exe` (Yuri's Revenge, patched) |
| Build path | `D:\ra2mdpost\` |
| Functions | **8,973** (all stripped — `FUN_00XXXXXX`) |
| Classes | **~160 unique** game classes (980+ with template instantiations) |
| RTTI entries | **1,111** type info strings |
| Exports | 1 (`entry` at `0x7CD80F`) |
| Source files | **74** original `.CPP` paths embedded as debug assert strings |

### Memory Segments

| Segment | Start | End | Size | Purpose |
|---|---|---|---|---|
| `.text` | `0x401000` | `0x7E0FFF` | **3.9 MB** | Executable code |
| `.rdata` | `0x7E1000` | `0x811FFF` | **196 KB** | Read-only data (vtables, RTTI, strings, constants) |
| `.data` | `0x812000` | `0xB79BE3` | **3.5 MB** | Mutable globals (game state, arrays, buffers) |
| `.rsrc` | `0xB7A000` | `0xC03FFF` | **552 KB** | Resources (dialogs, icons, cursors) |

---

## 1. Original Source Files (from debug assert strings)

The binary embeds 74 source file paths from `D:\ra2mdpost\`. These map directly to
subsystems:

### Core Engine
| File | Subsystem | Key Functions |
|---|---|---|
| `Conquer.CPP` | Main game loop | `Main_Game()` = `FUN_0048ccc0` |
| `MainLoop.CPP` | Frame dispatch | Main tick / frame loop |
| `Startup.CPP` | Initialization | WinMain, class registration |
| `Init.CPP` | Game init | Asset loading, theater setup |
| `Scenario.CPP` | Map/scenario load | `New_Scenario()` = `FUN_0052d9a0` |
| `Queue.CPP` | Command queue | Multiplayer lockstep commands |
| `Event.CPP` | Event dispatch | Network events, game events |
| `Session.CPP` | Session mgmt | Game mode, player state |

### Display / UI Hierarchy
| File | Subsystem | Class |
|---|---|---|
| `Display.CPP` | Isometric view | `DisplayClass` |
| `Radar.CPP` | Minimap | `RadarClass` |
| `Power.CPP` | Power bar | `PowerClass` |
| `Sidebar.CPP` | Production sidebar | `SidebarClass` |
| `Tactical.CPP` | Tactical rendering | `TacticalClass` / `Tactical` |
| `Options.CPP` | Game options | `OptionsCommandClass` |
| `wwmous.cpp` | Mouse cursor | `WWMouseClass` |
| `ToolTip.cpp` | Tooltips | `ToolTipManager` |

### Game Objects
| File | Subsystem | Class |
|---|---|---|
| `Techno.CPP` | Base unit logic | `TechnoClass` |
| `Infantry.CPP` | Infantry | `InfantryClass` |
| `Unit.CPP` | Vehicles/ships | `UnitClass` |
| `House.CPP` | Player/AI house | `HouseClass` |
| `AbsType.cpp` | Type system base | `AbstractTypeClass` |

### Weapons / Combat
| File | Subsystem |
|---|---|
| `Ion.cpp` | Ion cannon super weapon |
| `Super.CPP` | Super weapons |
| `Beacon.CPP` | Beacon placement |

### AI System
| File | Subsystem |
|---|---|
| `PlanMgr.cpp` | AI planning |

### Multiplayer / Network
| File | Subsystem |
|---|---|
| `Connect.CPP` | Network connection |
| `Multiplayer.cpp` | MP game modes |
| `MPLayer.CPP` | MP layer management |
| `MPScore.cpp` | MP scoring |
| `MPCoop.cpp` | Co-op campaign |
| `MPSiege.cpp` | Siege mode |
| `MPSiegeTeam.cpp` | Siege teams |
| `MPObserver.cpp` | Observer/spectator |
| `NetDlg.CPP` | Network dialogs |
| `netdlg2.cpp` | Network dialogs (cont.) |
| `netshare.cpp` | Shared network code |
| `NullDlg.CPP` | Null modem dialog |
| `NullMgr.CPP` | Null modem manager |
| `SerialEd.cpp` | Serial editor |
| `PhoneEd.cpp` | Phone editor |
| `ModemGst.cpp` | Modem guest |
| `ModemHst.cpp` | Modem host |
| `SendFile.CPP` | File transfer |

### UI / Menus
| File | Subsystem |
|---|---|
| `GameDlg.CPP` | Game dialogs |
| `GDlgSupp.cpp` | Dialog support |
| `GOptions.CPP` | Game options |
| `Skirmish.cpp` | Skirmish setup |
| `LoadDlg.CPP` | Load game dialog |
| `LdPrgMgr.cpp` | Loading progress |
| `Score.CPP` | Score screen |
| `stlscore.cpp` | Score (alt) |
| `CampScor.cpp` | Campaign score |
| `Credits.CPP` | Credits |
| `Egos.CPP` | Ego screen |
| `MSChoice.cpp` | Map selection choice |
| `MapSel.CPP` | Map selection |
| `MapGen.cpp` | Map generation |
| `ownrdraw.cpp` | Owner-drawn controls |
| `UICmnds.cpp` | UI commands |

### Westwood Online / World Domination
| File | Subsystem |
|---|---|
| `wonline.cpp` | Westwood Online |
| `WOLPersonaInformation.cpp` | Player profiles |
| `WorldDom.cpp` | World Domination Tour |
| `WDTGameOptions.cpp` | WDT options |
| `WDTProps.cpp` | WDT properties |
| `WDTSel.cpp` | WDT selection |
| `WDTTerr.cpp` | WDT territory |

### Misc
| File | Subsystem |
|---|---|
| `Restate.cpp` | Re-establish connection |
| `CD.CPP` | CD check / disk swap |
| `Dropship.cpp` | Dropship loading screen |
| `Ini.CPP` | INI parser |
| `coopcamp.cpp` | Co-op campaigns |

---

## 2. Complete Class Hierarchy

### Object Inheritance Chain (Deep OOP)

```
AbstractClass (36 bytes)
├─ 4 vtable pointers: IPersistStream, IRTTITypeInfo, INoticeSink, INoticeSource
├─ UniqueID (+0x10), Flags (+0x14), RefCount (+0x1C)
│
├─► ObjectClass (~176 bytes) — anything placeable on the map
│   ├─ Health (+0x6C), InLimbo (+0x81), IsSelected (+0x83)
│   ├─ Location XYZ (+0x9C..0xA4), OnBridge (+0x8C)
│   │
│   ├─► MissionClass (~216 bytes) — unit orders
│   │   ├─ CurrentMission, SuspendedMission
│   │   │
│   │   ├─► RadioClass (~272 bytes) — team/tether communication
│   │   │   ├─ Contact links for docking, tethering
│   │   │   │
│   │   │   ├─► TechnoClass (~1312 bytes, 0x520) — weapons, targeting, AI
│   │   │   │   ├─ Owner HouseClass* (+0x228)
│   │   │   │   ├─ Target (+0x2AC), Ammo (+0x2F4), RearmTimer (+0x2E4)
│   │   │   │   ├─ 8× CDTimerClass (IronCurtain, Chrono, etc.)
│   │   │   │   ├─ ILocomotion COM interface (+0x674)
│   │   │   │   │
│   │   │   │   ├─► FootClass (~1760 bytes, 0x6E0) — locomotion, pathing
│   │   │   │   │   ├─ MaxSpeed (+0x578), NavTarget (+0x5A4)
│   │   │   │   │   ├─ PathQueue (+0x5E0, 24 entries)
│   │   │   │   │   ├─ DriveTrackIndex (+0x684)
│   │   │   │   │   │
│   │   │   │   │   ├─► InfantryClass (~1776 bytes, 0x6F0)
│   │   │   │   │   │   ├─ DoType (+0x6C4), FearLevel (+0x6D4)
│   │   │   │   │   │   ├─ IsCrawling (+0x6DB), TerrainWalkState (+0x6E8)
│   │   │   │   │   │
│   │   │   │   │   ├─► UnitClass (~1824 bytes, 0x720)
│   │   │   │   │   │   ├─ Deploying, Harvesting, Rotating flags
│   │   │   │   │   │
│   │   │   │   │   └─► AircraftClass (~1752 bytes, 0x6D8)
│   │   │   │   │       ├─ Altitude, FlightState
│   │   │   │   │
│   │   │   │   └─► BuildingClass (~1824 bytes, 0x720)
│   │   │   │       ├─ BuildingType, AnimState, Factory
│   │   │   │       ├─ Power generation/drain
│   │   │   │
│   │   │   └─► (no direct instantiation)
│   │   └─► (no direct instantiation)
│   └─► (no direct instantiation)
│
├─► AbstractTypeClass — type definitions (parsed from INI)
│   ├─► ObjectTypeClass — base for placeable type data
│   │   ├─► TechnoTypeClass — weapons, armor, speed, cost, prerequisites
│   │   │   ├─► InfantryTypeClass — infantry-specific type data
│   │   │   ├─► UnitTypeClass — vehicle/ship-specific type data
│   │   │   ├─► AircraftTypeClass — aircraft-specific type data
│   │   │   └─► BuildingTypeClass — building-specific type data
│   │   │       ├─ WaterBound (+0x67C repurposes SpeedType)
│   │   │       ├─ Foundation, Power, Factory, Adjacent
│   │   ├─► BulletTypeClass
│   │   ├─► WarheadTypeClass
│   │   └─► WeaponTypeClass
│   │
│   ├─► AnimTypeClass — animation definitions
│   ├─► OverlayTypeClass — overlay type data
│   ├─► SmudgeTypeClass
│   ├─► TerrainTypeClass
│   ├─► IsometricTileTypeClass — theater tile data
│   ├─► ParticleTypeClass / ParticleSystemTypeClass
│   ├─► SuperWeaponTypeClass
│   ├─► HouseTypeClass — faction definitions
│   ├─► SideClass
│   ├─► TiberiumClass
│   ├─► CampaignClass
│   ├─► AITriggerTypeClass
│   ├─► ScriptTypeClass
│   ├─► TeamTypeClass
│   ├─► TaskForceClass
│   ├─► TagTypeClass / TriggerTypeClass
│   └─► VoxelAnimTypeClass
│
├─► AnimClass — explosions, visual effects (inherits ObjectClass)
├─► BulletClass — projectiles in flight
├─► IsometricTileClass — placed map tiles
├─► OverlayClass — placed overlays (ore, walls)
├─► SmudgeClass — terrain damage marks
├─► TerrainClass — placed terrain objects (trees, etc.)
├─► ParticleClass / ParticleSystemClass
├─► VoxelAnimClass
│
├─► HouseClass (~90,296 bytes, 0x160B8!) — player/AI state
│   ├─ Threat map (0x4204 entries), factory slots
│   ├─ Production queues, base plan nodes
│   ├─ Tech tree ownership, economy
│
├─► CellClass (~328 bytes, 0x148) — map grid cell
│   ├─ IsoTileTypeIndex (+0x38), OverlayIndex (+0x44)
│   ├─ LandType (+0xEC), Height (+0x11B)
│   ├─ OccupationFlags (+0x124), CellFlags (+0x140)
│   ├─ FirstObject (+0xE4), AltObject (+0xE8)
│
├─► FactoryClass — production queue instance
├─► TeamClass — AI team instance
├─► ScriptClass — AI script instance
├─► TagClass / TriggerClass / TActionClass / TEventClass — trigger system
├─► WaypointPathClass
├─► TubeClass — underground tunnel
└─► SuperClass — super weapon instance
```

### Display Class Chain (Single Inheritance, ~21 KB total)

```
GScreenClass
└─► MapClass              — cell grid, terrain data
    └─► DisplayClass       — isometric projection, object drawing
        └─► RadarClass     — minimap rendering
            └─► PowerClass — power bar display
                └─► SidebarClass — production sidebar
                    └─► TabClass    — tab strip
                        └─► ScrollClass  — scroll/pan handler
                            └─► MouseClass — cursor + input dispatch
```

This chain forms the **single global UI object** at `DAT_00887640`. Each layer
delegates to its parent, adding one UI feature. `MouseClass` is at the bottom
and handles all input events, which propagate up through the chain.

### Locomotion Classes (COM Interfaces)

| Class | CLSID | Movement Type |
|---|---|---|
| `DriveLocomotionClass` | `{4A582741-9839-11d1-B709-00A024DDAFD1}` | Tanks, wheeled |
| `WalkLocomotionClass` | `{4A582744-9839-11d1-B709-00A024DDAFD1}` | Infantry |
| `ShipLocomotionClass` | `{2BEA74E1-7CCA-11D3-BE14-00104B62A16C}` | Ships (=Drive+Float data) |
| `FlyLocomotionClass` | `{4A582746-9839-11d1-B709-00A024DDAFD1}` | Aircraft |
| `HoverLocomotionClass` | `{4A582742-9839-11d1-B709-00A024DDAFD1}` | Hovercraft |
| `JumpjetLocomotionClass` | `{92612C46-F71F-11D1-AC9F-006008055BB5}` | Rocketeers |
| `RocketLocomotionClass` | ... | V3 rockets |
| `MechLocomotionClass` | ... | Walkers |
| `TeleportLocomotionClass` | ... | Chrono units |
| `TunnelLocomotionClass` | ... | Tunnel network |
| `DropPodLocomotionClass` | ... | Drop pods |

Each locomotor is a COM object created via `CoCreateInstance()` with the CLSID
from the unit type's `Locomotor=` INI key. Stored at `FootClass+0x674` as
`ILocomotion*`. This is how Ares/Phobos can add new locomotors via DLL injection.

### Manager/Component Classes

| Class | Purpose |
|---|---|
| `SpawnManagerClass` | Carrier/Dreadnought sub-unit spawning |
| `SlaveManagerClass` | Slave miner control |
| `CaptureManagerClass` | Mind control (Yuri) |
| `TemporalClass` | Chrono erase weapon |
| `ParasiteClass` | Terror drone/squid attachment |
| `AirstrikeClass` | Boris airstrike painting |
| `BombClass` | Crazy Ivan / Terrorist bomb |
| `WaveClass` | Sonic/tesla weapon visual |
| `EBolt` | Tesla bolt rendering |
| `RadBeam` | Radiation beam visual |
| `LaserDrawClass` | Prism/laser weapon rendering |
| `DiskLaserClass` | Yuri disk laser |
| `RadSiteClass` | Radiation contamination |
| `EMPulseClass` | EMP effect |
| `IonBlastClass` | Ion cannon effect |
| `LightSourceClass` | Dynamic lighting |
| `AlphaShapeClass` | Chrono/cloak alpha blend |
| `BounceClass` | Physics bounce (debris) |
| `FoggedObjectClass` | Fog of war remembered objects |

---

## 3. Main Game Loop

### Entry Point: WinMain (`FUN_006bb9a0`, 1826 lines)

```
WinMain(hInstance)
├─ GetCurrentThreadId()
├─ Register heap pools via FUN_004068e0 (one per class, ~30 calls)
│  ├─ Pool(0x687, 0x720)   → BuildingClass, 1824 bytes each
│  ├─ Pool(0x68b, 0x148)   → CellClass, 328 bytes each
│  ├─ Pool(0x68d, 0x160B8) → HouseClass, 90,296 bytes each
│  ├─ Pool(0x681, 0x6D8)   → AircraftClass, 1752 bytes each
│  └─ ... (30+ class pools)
├─ Register COM class factories (TClassFactory<> for each class)
│  ├─ CoRegisterClassObject for DriveLocomotionClass
│  ├─ CoRegisterClassObject for WalkLocomotionClass
│  ├─ CoRegisterClassObject for ShipLocomotionClass
│  └─ ... (11 locomotor types + dozens of game classes)
├─ Create mutex (prevent multiple instances)
├─ Initialize DirectDraw, DirectSound
├─ Load MIX archives
├─ Parse rules.ini / rulesmd.ini / art.ini / artmd.ini
├─ Call Main_Game() → FUN_0048ccc0
└─ Cleanup
```

### Main_Game (`FUN_0048ccc0`, 125 lines)

```
Main_Game()
├─ FUN_007ca650()  — CRT/exception init
├─ FUN_0052ba60()  — check installer/version
│
├─ OUTER LOOP: while New_Scenario() succeeds:
│  ├─ DAT_00a8e7ac = 0      (clear map editor mode)
│  ├─ FUN_004a3c30()         (display init)
│  ├─ FUN_0054f720()         (theater init — load tiles/palettes)
│  ├─ FUN_006c87f0()         (network: if online mode)
│  ├─ FUN_0069bab0()         (scenario start — spawn units, init AI)
│  │
│  ├─ INNER LOOP (per-frame):
│  │  ├─ FUN_0055d360()      ★ MAIN TICK — all game logic
│  │  ├─ FUN_0055cfd0()      — check quit condition
│  │  ├─ FUN_0048c8b0()      ★ STATE MACHINE — render + UI
│  │  └─ repeat until quit
│  │
│  ├─ FUN_0072dfb0()         (cleanup audio)
│  ├─ FUN_0069bb40()         (scenario end — cleanup)
│  ├─ Score/stats screens (mode-dependent)
│  └─ repeat for next scenario
│
└─ Cleanup display object
```

### State Machine (`FUN_0048c8b0`)

The global `DAT_00a8eda0` controls the current game state:

| State | Value | Handler | Description |
|---|---|---|---|
| None | 0 | (return) | No active state |
| **Gameplay** | 1 | `switchD_caseD_1` | Main gameplay — render + input |
| Surrender | 2 | `FUN_005c60d0` | Surrender dialog, queues command |
| Options | 3 | `FUN_004f1840` | Pause menu (save/load/options/quit) |
| Briefing | 4 | `FUN_005fbef0` | Mission briefing screen |
| Loading | 5 | `FUN_004e1d00` | Load game screen → transitions to 1 |
| Score | 6 | `FUN_006b6230` | Score/stats screen |
| Movie | 7 | `FUN_0077d840` | FMV playback |
| ?? | 8 | `FUN_006586d0` | (unknown — campaign select?) |
| MapSelect | 9 | `FUN_0065f520` | Map selection screen |

### Main Tick (`FUN_0055d360`, 485 lines)

The per-frame update function. This is the heart of the engine:

```
Main_Tick()
├─ PHASE 1: Frame Timing
│  ├─ If paused (DAT_00a8ed80 == 0): Sleep(500), process Windows messages
│  ├─ Read timeGetTime() for frame budget
│  ├─ Compute desired frame rate from game speed setting
│  ├─ For online games: adaptive frame timing based on network latency
│  │   ├─ If player latency > 25% of budget → add 10ms
│  │   ├─ If player latency > 50% → add another 10ms
│  │   └─ If player latency > 75% → add another 10ms
│
├─ PHASE 2: Network Sync (multiplayer only)
│  ├─ FUN_005d4d50()  — process network messages (Call_Back)
│  ├─ FUN_0048d080()  — network service loop
│  ├─ FUN_0053b560()  — process queued events
│  ├─ DAT_00887324 vtable+0x5C — display update callback
│  ├─ FUN_004f4480()  — render frame
│  └─ FUN_0055e160()  — (post-render)
│
├─ PHASE 3: Game Logic (if game is running, DAT_00a8eda0 == 0)
│  ├─ FUN_0055dee0()  ★ LogicClass::AI() — tick ALL game objects
│  │  ├─ For each object in each layer:
│  │  │   ├─ AbstractClass::AI()  — base tick
│  │  │   ├─ ObjectClass::AI()    — position, animation
│  │  │   ├─ TechnoClass::AI()   — targeting, weapons, timers
│  │  │   ├─ FootClass::AI()     — pathfinding, locomotion
│  │  │   ├─ InfantryClass::AI() / UnitClass::AI() / etc.
│  │  │   └─ Each subclass adds its own per-tick behavior
│  │  ├─ Process triggers and scripts
│  │  └─ Update scores, check win/loss
│  │
│  ├─ FUN_0055f1e0()  — AI house tick (if enabled)
│  ├─ FUN_00542520()  — periodic network keepalive (every 8th frame, online)
│  ├─ FUN_004d2370()  — Map::Logic() — cell updates, tiberium growth
│  └─ FUN_004f4480()  — render frame
│
├─ PHASE 4: Lockstep Sync (if multiplayer flags set)
│  ├─ Send frame data: random seed hash, command checksums
│  ├─ Receive and validate peer data
│  ├─ On desync: FUN_0048dc90() — desync handler
│  └─ Execute received commands from peers
│
└─ Return: 1 if game ended, 0 if continue
```

---

## 4. Key Global Variables

### Core State

| Address | Type | Name | Used In |
|---|---|---|---|
| `DAT_00a8ed84` | `int` | **g_CurrentFrameCounter** | 65+ subsystems — THE game tick counter |
| `DAT_008871e0` | `RulesClass*` | **Rules singleton** | 58+ subsystems — all INI data |
| `DAT_00a8b238` | `int` | **GameMode** | 49+ subsystems (0=SP, 1=Skirmish, 2=LAN, 3=WOL, 4=TCP) |
| `DAT_00a83d4c` | `HouseClass*` | **LocalPlayer** | 35+ subsystems — current human player |
| `DAT_00a8022c` | `HouseClass[]` | **AllHouses** | 35+ subsystems — all 35 house slots |
| `DAT_00887640` | `MouseClass*` | **Display chain** | Rendering, input — the single global UI object |
| `DAT_00887324` | `TacticalClass*` | **Tactical** | Isometric rendering, viewport |
| `DAT_00a8eda0` | `int` | **GameState** | State machine (0-9) |
| `DAT_00a8ed80` | `char` | **GameRunning** | Pause/resume flag |
| `DAT_00a8e9a0` | `char` | **GameActive** | Master enable flag |
| `DAT_00a8e7ac` | `int` | **MapEditorMode** | Disables gameplay checks when 1 |

### Rendering

| Address | Type | Name | Purpose |
|---|---|---|---|
| `DAT_00887368` | `ptr` | Audio system | Sound engine state |
| `DAT_008a0360` | `LayerClass[5]` | Display layers | Ground/Building/Air/Effect/Top |
| `DAT_0087f924` | `CellClass*` | Cell array base | Map grid (512×512 max) |
| `DAT_00887314` | `DSurface*` | Primary surface | DirectDraw render target |

### Network

| Address | Type | Purpose |
|---|---|---|
| `DAT_00a802c8` | `int` | Command queue count (max 128) |
| `DAT_00a802d0` | `int` | Command queue write index |
| `DAT_00a802d4` | `byte[128×0x6F]` | Command buffer (128 slots, 111 bytes each) |
| `DAT_00a83a54` | `DWORD[128]` | Command timestamps (timeGetTime) |
| `DAT_00a8b550` | `int` | Network frame budget (ms) |

### Data Tables

| Address | Type | Purpose |
|---|---|---|
| `DAT_0089ea40` | `float[12][9]` | Speed/terrain table (from rules.ini) |
| `DAT_0082a594` | `int[12][8]` | Passability matrix (hardcoded) |
| `DAT_007f2a40` | `byte[72×12]` | Drive track descriptors |
| `DAT_007f2960` | `byte[16×16]` | Drive track step arrays (pointers) |
| `DAT_0089f688` | `int[8×2]` | 8-direction cell offsets (dx/dy) |
| `DAT_00b077f8` | `int[3]` | Null-coordinate sentinel (XYZ) |
| `DAT_00b0782c` | `int` | Bridge Z-offset constant |

---

## 5. Subsystem Architecture

### A. Object Lifecycle

```
1. INI PARSING → TypeClass created (InfantryTypeClass, UnitTypeClass, etc.)
   ├─ Stored in global DynamicVectorClass per type
   └─ Indexed by string ID from INI section name

2. OBJECT CREATION → Instance allocated from class-specific heap pool
   ├─ FUN_004068e0 registered pool sizes at startup
   ├─ Constructor chains up inheritance: Abstract→Object→Mission→Radio→Techno→Foot→Unit
   ├─ 4 vtable pointers set (IPersistStream, RTTI, INoticeSink, INoticeSource)
   ├─ COM locomotor created: CoCreateInstance(CLSID) → ILocomotion*
   └─ Object placed in appropriate DynamicVectorClass and LayerClass

3. PER-TICK UPDATE → AI() virtual method chain
   ├─ Called from LogicClass during Main_Tick
   ├─ Each inheritance level adds behavior
   └─ Timers (CDTimerClass) auto-decrement

4. SERIALIZATION → IPersistStream::Save() / Load()
   ├─ Writes all fields in order down the inheritance chain
   ├─ SwizzleManagerClass resolves pointer fixups on load
   └─ Used for save/load AND network state sync
```

### B. Rendering Pipeline

```
Frame render (from Main_Tick → FUN_004f4480):
│
├─ TacticalClass::Draw (FUN_006d3d10, ~3643 bytes)
│  ├─ Update viewport (camera position, scroll)
│  ├─ Dirty rectangle tracking
│  │
│  ├─ LAYER 0: Terrain
│  │  ├─ FUN_004d1890 — draw isometric tiles
│  │  ├─ Tile index → IsometricTileTypeClass → .tmp pixel data
│  │  └─ Height offset per cell
│  │
│  ├─ LAYER 1: Overlays
│  │  ├─ FUN_006d7c00 — walls, ore, gems, pavement
│  │  └─ Overlay frame based on damage/state
│  │
│  ├─ LAYER 2-3: Game Objects (sorted by screen Y)
│  │  ├─ For each object in layer:
│  │  │  ├─ VXL units: ILocomotion::Draw_Matrix → voxel rasterize
│  │  │  ├─ SHP units: frame index → palette remap → blit
│  │  │  └─ Z-buffer write per pixel
│  │  │
│  │  ├─ ConvertClass handles all palette remapping:
│  │  │  ├─ Owner color (house remap)
│  │  │  ├─ Damage state (darker tint)
│  │  │  ├─ Cloak (translucency)
│  │  │  ├─ Disguise (enemy appearance)
│  │  │  └─ Special FX (chrono, warp, etc.)
│  │  │
│  │  └─ 100+ Blit* template classes for every combination:
│  │     BlitTransXlatZReadWrite, RLEBlitTransLucent50Alpha, etc.
│  │
│  ├─ LAYER 4: Effects
│  │  ├─ AnimClass instances (explosions, smoke)
│  │  ├─ LaserDrawClass, WaveClass, EBolt, RadBeam
│  │  └─ ParticleSystemClass
│  │
│  └─ LAYER 5: UI overlays
│     ├─ Selection boxes, health bars
│     ├─ Shroud/fog edges (FUN_004801f0)
│     └─ Smudges (FUN_006d6d10)
│
├─ SidebarClass::Draw — production queue cameos
├─ RadarClass::Draw — minimap
├─ PowerClass::Draw — power bar
└─ MouseClass::Draw — cursor sprite
```

### C. Combat System

```
TechnoClass::Fire_Weapon()
├─ Select weapon index (primary/secondary)
├─ Check ammo, rearm timer, range, line of sight
├─ Compute barrel position (FLH offsets from artmd.ini)
├─ Create BulletClass via COM factory
│
BulletClass::AI() (FUN_004666e0, 6,422 bytes)
├─ Update trajectory: velocity, gravity, wind
├─ Cell-by-cell movement, checking for obstacles
├─ On impact → Warhead detonation
│
WarheadTypeClass::Detonate (FUN_004690b0, 4,692 bytes)
├─ Apply area damage via FUN_00489280
├─ Iterate cells within warhead radius
├─ For each object: apply Verses[] armor modifier → subtract HP
├─ Spawn impact animations
└─ Shrapnel sub-projectiles (if any)
```

### D. AI System

```
HouseClass::AI() — per-house, per-tick
├─ Economy: track income, manage spending
├─ Production: decide what to build next
│  ├─ Threat assessment (0x4204-entry threat map)
│  ├─ DiscreteDistributionClass<BuildingTypeClass> for probabilistic choices
│  └─ BuildChoiceClass ranking
├─ Base planning: place buildings
│  ├─ BaseNodeClass array (16 bytes per node)
│  └─ Adjacent check, threat avoidance
├─ Team management: create teams, assign scripts
│  ├─ TeamClass → TeamTypeClass → TaskForceClass → ScriptTypeClass
│  └─ ScriptClass::Execute — step through AI script commands
├─ Trigger system:
│  ├─ AITriggerTypeClass — condition → action mapping
│  ├─ TriggerClass evaluates TEventClass conditions
│  └─ TActionClass executes results
└─ Target acquisition:
   ├─ Scan nearby enemies
   ├─ NavalTargeting priority
   └─ Anti-air/anti-ground classification
```

### E. Pathfinding

```
FootClass pathfinding pipeline:
│
├─ Path Request:
│  ├─ Convert destination to cell coords
│  ├─ Zone pre-check: FUN_0042c290 — is destination reachable at all?
│  │  (SubzoneConnectionStruct, ZoneConnectionClass connectivity graph)
│  └─ If unreachable → abort immediately
│
├─ A* Search: FUN_0042c900
│  ├─ Max iterations: 65,527 (0xFFF7)
│  ├─ Heuristic: octile distance
│  ├─ Cell cost: passability matrix lookup
│  │  ├─ MovementZone → row in DAT_0082a594
│  │  ├─ LandType → column
│  │  └─ Result: 1=pass, 2=block, 3=OOB
│  ├─ Speed modifier: DAT_0089ea40[SpeedType × 9 + LandType]
│  ├─ Slope modifier: uphill/downhill from rules
│  └─ Output: direction commands → PathQueue (+0x5E0)
│
├─ Path Execution:
│  ├─ ILocomotion::Process() — per-tick
│  ├─ Drive track selection: facing × 8 + target_facing
│  ├─ Drive track step: FUN_006a05f0 (5,737 bytes)
│  │  ├─ Acceleration/deceleration curves
│  │  ├─ Sub-tick step loop (7 sub-ticks per step)
│  │  └─ Cell boundary transitions → occupation bit updates
│  └─ Path re-planning every 24 steps or on blockage
│
└─ Passability Matrix (hardcoded at DAT_0082a594):
   12 zones × 8 land types = 96 int32 entries
   Zone 10 (Ship): only water passable
   Zone 3 (Hover): all terrain except walls
   Zone 0 (Foot): only clear passable
```

---

## 6. DLL Dependencies

| DLL | Purpose |
|---|---|
| `KERNEL32.DLL` | File I/O, threading, memory, process management |
| `USER32.DLL` | Window management, message pump, dialogs |
| `GDI32.DLL` | Font rendering, basic 2D graphics |
| `ADVAPI32.DLL` | Registry access, security |
| `DDRAW.DLL` | DirectDraw 7 — primary rendering API |
| `DSOUND.DLL` | DirectSound — audio playback |
| `OLE32.DLL` | COM runtime — `CoCreateInstance`, `CoRegisterClassObject` |
| `OLEAUT32.DLL` | OLE Automation — `OleSaveToStream` |
| `BINKW32.DLL` | Bink video playback (FMV cutscenes) |
| `WSOCK32.DLL` | Winsock 1.1 — network (LAN/online) |
| `WINMM.DLL` | `timeGetTime` — frame timing |
| `SHELL32.DLL` | Shell utilities |
| `VERSION.DLL` | Version info queries |
| `IMM32.DLL` | Input Method Manager |
| `COMCTL32.DLL` | Common controls |

---

## 7. Class Sizes (from Heap Pool Registration)

Extracted from `WinMain` (`FUN_006bb9a0`) — each `FUN_004068e0` call registers a
fixed-size heap pool for one class type:

| Type ID | Size (bytes) | Likely Class |
|---|---|---|
| 0x67C | 152 | SmallObject (timer? event?) |
| 0x67D | 76 | SmallObject |
| 0x67E | 76 | SmallObject |
| 0x67F | 19 | TinyObject |
| 0x680 | 3,608 | BuildingTypeClass or large TypeClass |
| 0x681 | 1,752 | **AircraftClass** (0x6D8) |
| 0x682 | 3,600 | Large TypeClass |
| 0x683 | 272 | RadioClass-sized |
| 0x684 | 456 | AnimClass? |
| 0x685 | 888 | Medium TypeClass |
| 0x686 | 120 | BulletClass? |
| 0x687 | 1,824 | **BuildingClass** (0x720) |
| 0x688 | 6,040 | Large TypeClass (BuildingTypeClass?) |
| 0x689 | 352 | BulletTypeClass? |
| 0x68A | 760 | Medium TypeClass |
| 0x68B | 328 | **CellClass** (0x148) |
| 0x68C | 116 | SmallClass |
| 0x68D | 90,296 | **HouseClass** (0x160B8) |
| 0x68E | 432 | AnimTypeClass? |
| 0x68F | 1,776 | **InfantryClass** (0x6F0) |
| 0x690 | 3,792 | InfantryTypeClass? |
| 0x691 | 24 | **AbstractClass** (0x18) |
| 0x692 | 21,868 | **Display chain** (MouseClass) (0x556C) |

---

## 8. Template / Container Infrastructure

The engine uses two core container templates (no STL):

### VectorClass<T>
- Simple contiguous array with count + capacity
- Heap-allocated backing store
- No bounds checking in release builds

### DynamicVectorClass<T> (extends VectorClass)
- Auto-growing array (doubles capacity)
- Used for ALL game object collections
- ~95 distinct instantiations in the binary

### Key Global Vectors

| Vector Type | Global | Purpose |
|---|---|---|
| `DynamicVectorClass<InfantryClass*>` | `DAT_00a83ce4` | All infantry on map |
| `DynamicVectorClass<UnitClass*>` | near above | All vehicles on map |
| `DynamicVectorClass<AircraftClass*>` | near above | All aircraft |
| `DynamicVectorClass<BuildingClass*>` | near above | All buildings |
| `DynamicVectorClass<BulletClass*>` | — | All projectiles in flight |
| `DynamicVectorClass<AnimClass*>` | — | All active animations |
| `DynamicVectorClass<HouseClass*>` | `DAT_00a8022c` | All houses (35 max) |
| `DynamicVectorClass<TeamClass*>` | — | All AI teams |
| `DynamicVectorClass<TriggerClass*>` | — | All active triggers |
| `DynamicVectorClass<FactoryClass*>` | — | All production queues |
| `DynamicVectorClass<SpawnManagerClass*>` | — | All spawn managers |
| `DynamicVectorClass<SlaveManagerClass*>` | — | All slave managers |
| `DynamicVectorClass<CaptureManagerClass*>` | — | All mind controllers |
| `DynamicVectorClass<LaserDrawClass*>` | — | All laser draws |
| `DynamicVectorClass<WaveClass*>` | — | All wave visuals |
| `DynamicVectorClass<ParticleSystemClass*>` | — | All particle systems |
| `DynamicVectorClass<LightSourceClass*>` | — | All dynamic lights |
| `DynamicVectorClass<RadSiteClass*>` | — | All radiation sites |

---

## 9. COM Architecture Details

### Interface Hierarchy per Object

Every game object carries **4 interface pointers** at the start of its memory:

```
+0x00: IPersistStream vtable  — serialization (Save/Load to stream)
+0x04: IRTTITypeInfo vtable   — runtime type identification
+0x08: INoticeSink vtable     — receive notifications
+0x0C: INoticeSource vtable   — send notifications
```

### TClassFactory<T>

Each game class has a `TClassFactory` registered at startup via
`CoRegisterClassObject`. This allows `CoCreateInstance(CLSID)` to create objects.

There are **43 registered factory classes** in the binary (one per
serializable/COM-creatable game class). This is what enables:
- Save/load (IPersistStream creates objects by CLSID from save file)
- Locomotor creation (CoCreateInstance with Locomotor CLSID from INI)
- Network object creation (peer sends CLSID, receiver creates matching object)

### ILocomotion Interface (40 methods)

The locomotion COM interface provides a standardized API for movement:

| Index | Method | Purpose |
|---|---|---|
| 0-2 | QueryInterface/AddRef/Release | COM lifecycle |
| 3 | Link_To_Object | Attach to owning techno |
| 4 | Is_Moving | Movement query |
| 5 | Destination | Get destination coords |
| 6 | Head_To_Coord | Get next waypoint |
| 7 | Can_Enter_Cell | Cell passability check |
| 9 | Draw_Matrix | Voxel transform (body + turret) |
| 10 | Shadow_Matrix | Shadow transform |
| 16 | **Process** | Per-tick movement update |
| 17 | Move_To | Set destination |
| 18 | Stop_Moving | Clear movement |
| 19 | Do_Turn | Face direction |
| 20 | Unlimbo | Initialize on map |
| 28 | Force_Track | Force movement path |
| 29 | In_Which_Layer | Layer classification |
| 38 | Is_Surfacing | Submarine state query |
| 39 | Mark_All_Occupation_Bits | Cell occupation |

---

## 10. What Can Be Mapped vs. What Can't

### Fully Mappable via Ghidra MCP

- **All 8,973 functions** can be decompiled (just unnamed)
- **All vtables** can be read and methods traced
- **All data tables** can be inspected (passability, speed, drive tracks, etc.)
- **All global variables** can be identified by xref tracing
- **All class layouts** can be reconstructed from constructor analysis
- **All COM interfaces** can be fully enumerated
- **All string references** (1,111 RTTI + debug paths + INI keys)

### Requires Significant Effort

- **Function naming**: 8,973 functions × manual classification
  (estimated ~200 hours of focused work to name the important ~2,000)
- **Full field maps**: each class needs constructor + method analysis
  to map every byte offset to a named field
- **Algorithm documentation**: complex functions like pathfinding (65K iterations),
  warhead detonation (4,692 bytes), combat AI need line-by-line annotation

### Cannot Be Determined from Binary Alone

- **Original variable names** (stripped, only struct field offsets remain)
- **Original comments** (not compiled into binary)
- **Design intent** (why certain magic numbers were chosen)
- **Unused/dead code paths** (hard to distinguish from rarely-used code)
- **Compiler optimizations** (inlined functions lose their identity)

### Practical Approach

The subsystem-by-subsystem approach used for this engine is optimal:
1. **Identify subsystem** from class hierarchy + source file names
2. **Trace vtable** to find all methods of a class
3. **Decompile key methods** (AI, Process, Fire, Draw)
4. **Follow xrefs** to understand data flow
5. **Cross-reference with INI** to validate constants
6. **Document** in focused research docs (like NAVAL_SYSTEM_RESEARCH.md)

Each subsystem takes 2-4 hours of focused Ghidra work to fully document.
At ~50 subsystems, full coverage would take ~100-200 hours of analysis.

---

## Key Addresses Quick Reference

### Entry Points
| What | Address |
|---|---|
| WinMain | `0x6BB9A0` |
| Main_Game | `0x0048CCC0` |
| Main_Tick | `0x0055D360` |
| State Machine | `0x0048C8B0` |
| New_Scenario | `0x0052D9A0` |

### Object AI Methods (via vtable dispatch)
| What | Address Range | Notes |
|---|---|---|
| LogicClass tick | `0x55DEE0` area | Iterates all objects |
| TechnoClass AI | varies by vtable | Targeting, weapons, timers |
| FootClass AI | varies by vtable | Pathfinding, locomotion |
| BuildingClass AI | varies by vtable | Production, animation |

### Key Subsystem Functions
| What | Address | Size |
|---|---|---|
| Main movement AI | `0x6A1C80` | 8,470 bytes |
| Drive track execution | `0x6A05F0` | 5,737 bytes |
| BulletClass::AI | `0x4666E0` | 6,422 bytes |
| Warhead detonate | `0x4690B0` | 4,692 bytes |
| Tactical::Draw | `0x6D3D10` | 3,643 bytes |
| A* pathfind | `0x42C900` | large |
| Cell passability | `0x47C620` | building placement |
