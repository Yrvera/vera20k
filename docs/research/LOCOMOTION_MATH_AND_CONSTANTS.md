# Locomotion Math & Constants — Ghidra Decompilation Report

Source: Ghidra MCP (gamemd.exe) + decompiled C files at `C:\Users\enok\Documents\gidra\`

---

## 1. Locomotor CLSID Table

| CLSID | Class | Constructor | ILocoVTable | Units | Ghidra File |
|-------|-------|-------------|-------------|-------|-------------|
| `{4A582741-...-00A024DDAFD1}` | **DriveLocomotionClass** | `0x4AF540` | `0x7E7EB0` | Grizzly, Prism Tank, etc. | 034 |
| `{4A582742-...-00A024DDAFD1}` | **HoverLocomotionClass** | `0x513C20` | `0x7EACFC` | Robot Tank | 053 |
| `{4A582743-...-00A024DDAFD1}` | **TunnelLocomotionClass** | `0x728A00` | `0x7F5A24` | (unused) | 136 |
| `{4A582744-...-00A024DDAFD1}` | **WalkLocomotionClass** | `0x75AA90` | `0x7F69F8` | GI, Dog, Engineer | 145 |
| `{4A582745-...-00A024DDAFD1}` | **DropPodLocomotionClass** | `0x4B5AB0` | `0x7E8278` | (unused) | 035 |
| `{4A582746-...-00A024DDAFD1}` | **FlyLocomotionClass** | `0x4CC9A0` | `0x7E89F4` | Harrier, Kirov | 038 |
| `{4A582747-...-00A024DDAFD1}` | **TeleportLocomotionClass** | `0x718000` | `0x7F5000` | Chrono Legionnaire, CMIN | 133 |
| `{55D141B8-DB94-11d1-AC98-006008055BB5}` | **MechLocomotionClass** | `0x5AFEF0` | `0x7EDB6C` | (unused in YR) | 074 |
| `{2BEA74E1-7CCA-11d3-BE14-00104B62A16C}` | **ShipLocomotionClass** | `0x69EC50` | `0x7F2D8C` | Destroyer, Aegis | 113 |
| `{92612C46-F71F-11d1-AC9F-006008055BB5}` | **JumpjetLocomotionClass** | `0x54AC40` | `0x7ECD68` | Rocketeer | 061 |
| `{B7B49766-E576-11d3-9BD9-00104B972FE8}` | **RocketLocomotionClass** | `0x661EC0` | `0x7F0B1C` | V3 Rocket | 104 |

**Ghidra report misidentifications:**
- Report 061 calls its class "WalkLocomotionClass" — it is actually **JumpjetLocomotionClass**
- Report 053 calls its class "FlyLocomotionClass" — it is actually **HoverLocomotionClass**
- Report 145 calls its class "unknown locomotor" — it is actually **WalkLocomotionClass**

---

## 2. Coordinate Systems

| System | Description |
|--------|-------------|
| **Leptons** | Sub-cell precision. 1 cell = 256 leptons. 3D signed ints (X, Y, Z). |
| **Cells** | Discrete grid. `cell = lepton >> 8` (divide by 256). |
| **Height levels** | Signed char at cell offset `+0x11B`. Pixel offset = `level * (-15)`. |
| **Cliff threshold** | 4 height levels difference = impassable cliff. |

---

## 3. Speed & Acceleration — Common Formulas

### 3.1 Speed Ramping (Drive, Walk, Ship)

All ground/foot locomotors share this pattern:

```c
if (current_speed < target_speed)
    current_speed += acceleration;
if (current_speed > target_speed)
    current_speed -= acceleration * 1.5;  // 1.5x deceleration (DAT_007e48f0 = 1.5)
current_speed = clamp(current_speed, 0, max_speed);
```

- Acceleration constant: `DAT_007e48f0` (Walk), per-type for Drive
- Deceleration is always **1.5x** the acceleration rate (verified: `DAT_007e48f0` = IEEE 754 double 1.5)

### 3.2 Slope Penalties

Lookup table at `DAT_0089ea40`: 9 floats per locomotion type, indexed by terrain slope class.

Rules multipliers (offsets from `DAT_008871e0` = RulesClass global):
| Offset | INI Key | Purpose |
|--------|---------|---------|
| `+0x768` | (internal) | Uphill speed multiplier |
| `+0x770` | (internal) | Downhill speed bonus |
| `+0x778` | (internal) | Additional uphill factor |
| `+0x780` | (internal) | Additional downhill factor |

### 3.3 Direction Tables

- **8-direction lepton offsets**: `DAT_0089f6d8` / `DAT_0089f6dc` — 8 entries of (dx, dy), 8 bytes each
- **8-direction cell offsets**: `DAT_0089f688` — 8 entries of (dx, dy) shorts
- **Track flags**: `DAT_007e7b30` — indexed by track index, bits control mirroring/flipping

---

## 4. Drive Locomotion (Ground Vehicles)

**Ghidra files:** 034 (constructor + track system), 113 (ship/drive tick)

### 4.1 Drive Track System

Track table base: `DAT_007e7b28` — 12 bytes per entry:
- Byte 0: forward track index
- Byte 1: reverse track index
- Byte 4: direction byte
- Byte 8+: flags

Track waypoint data base: `DAT_007e7a28` — outer table entries are **16 bytes each** (stride 0x10);
first field is a pointer to the waypoint array. Each waypoint within the track is **12 bytes** (dx=4, dy=4, heading=4),
accessed via `waypoint_ptr + step_index * 0xc`. The doc previously said "12-byte entries" here, which conflicted
with the `* 0x10` stride — corrected 2026-05-28: was "12-byte entries"; binary shows outer stride is 16 (0x10),
inner waypoint stride is 12 (0xc) via `DriveLocomotionClass__Process_Drive_Track` at 0x4B0F20
— ROOT_CAUSE: OFFSET_RETYPED_WRONG (two strides conflated).

**Track transformation flags** (at `DAT_007e7b30`):
| Bit | Effect |
|-----|--------|
| 1 | Swap X/Y, negate heading by `0x40` |
| 2 | Negate X, negate heading |
| 4 | Negate Y, subtract `0x80` from heading |
| 8 | Cell-transition trigger |

**Movement budget**: subtract 7 per track step consumed per frame.

### 4.2 Bridge Detection in Track Movement

```c
if (cell_flags & 0x100) {  // bridge present
    compare height at +0x11B
    add height offset DAT_008a07c4 when on bridge
}
```

---

## 5. Fly Locomotion (Aircraft)

**Ghidra files:** 038 (Harrier/Kirov), 053 (Hover — misidentified as Fly)

### Acceleration Formula

```c
target_speed = computed_from_distance;
// delta is a hardcoded constant 0.1 (DAT_007e3860 = IEEE 754 double 0.1)
// NOT max_speed / (accel_factor * 60) as previously documented
delta = 0.1;  // corrected 2026-05-28: was "max_speed / (accel_factor * 60)";
              // binary DAT_007e3860 = 0.1, confirmed via FlyLocomotionClass__Process 0x4CD600
              // ROOT_CAUSE: INFERENCE_HARDENED (formula was inferred, not read from binary)

if (current_speed < target_speed)
    current_speed += delta;
if (current_speed > target_speed)
    current_speed -= delta;  // symmetric deceleration (same delta = 0.1)
```

**Key rule fields:**
| Offset from RulesClass | Purpose |
|------------------------|---------|
| `+0x16B8` | Speed cap (UNVERIFIED offset — not confirmed in this session) |
| `+0x5F0` | Acceleration factor (UNVERIFIED offset — not confirmed in this session) |

**Gravity-assist bonus** (UNVERIFIED): allegedly up to +1/3 speed when flying downhill.
Previously attributed to `FUN_0055a930`, but that address is `LocomotionClass::Is_Powered`
(a trivial getter) — the real source for this claim has not been identified. Treat as
inference until verified; may not exist in the binary.

**Descent slowdown zones** (distance-based, NOT tick-counts):
- threshold 0x14 (20) leptons/units from destination: cap speed to 20
- threshold 0x32 (50) leptons/units: cap speed to 50
- corrected 2026-05-28: previously said "ticks"; binary shows these are distance thresholds
  (`iVar17 = distance - approach_distance` compared against 0x14 / 0x32), confirmed via
  `FlyLocomotionClass__Process` at 0x4CD600 — ROOT_CAUSE: INFERENCE_HARDENED

---

## 6. Walk Locomotion (Infantry)

**Ghidra file:** 061 (misidentified — actually JumpjetLocomotionClass in 061; Walk is in 145)

7-state machine. Speed ramping identical to drive (linear accel, 1.5x decel).
Acceleration constant at `DAT_007e48f0`.

---

## 7. Teleport Locomotion (Chrono)

**Ghidra file:** 133

### 7.1 TeleportLocomotionClass Layout

```
+0x00: IUnknown vtable      (0x7F50CC)
+0x04: ILocomotion vtable    (0x7F5000)
+0x18: IPiggyback vtable     (0x7F4FDC)
+0x1C: Source coords         (3 ints — X, Y, Z)
+0x28: Destination coords    (3 ints — X, Y, Z)
+0x34: State flag byte
+0x35: Phase flag 1
+0x36: Phase flag 2
+0x38: Timer
+0x3C: Cached game frame
+0x44: Counter 1
+0x48: Counter 2
```

Invalid/sentinel coordinate: `(DAT_00b0ebf8, DAT_00b0ebfc, DAT_00b0ec00)`.

### 7.2 Chrono Constants — RulesClass Field Map

Parsed from `[General]` in rules(md).ini by `FUN_0066d530`:

| INI Key | String Addr | RulesClass Offset | Type | Default | Purpose |
|---------|-------------|-------------------|------|---------|---------|
| `ChronoDelay` | `0x83c714` | `+0xBEC` | int | — | Base chronosphere delay |
| `ChronoReinfDelay` | `0x83c700` | `+0xBF0` | int | — | Chrono reinforcement delay |
| `ChronoDistanceFactor` | `0x83c6e8` | `+0xBF4` | int | 48 | Leptons per frame of delay |
| `ChronoTrigger` | `0x83c6d8` | `+0xBF8` | bool | true | Enable distance-based delay |
| `ChronoMinimumDelay` | `0x83c6c4` | `+0xBFC` | int | 16 | Minimum frames in transit |
| `ChronoRangeMinimum` | `0x83c6b0` | `+0xC00` | int | — | Below this distance, always use MinDelay |
| `ChronoHarvTooFarDistance` | `0x83c464` | `+0xD7C` | int | 50 | Max cells for CMIN chrono-return |

**Assembly confirmation** (from `FUN_0066d530`):
```asm
0066fae3: PUSH 0x83c714          ; "ChronoDelay"
0066faeb: CALL 0x005276d0        ; ReadInt
0066faf0: MOV [ESI+0xBEC], EAX

0066fb02: PUSH 0x83c700          ; "ChronoReinfDelay"
0066fb0a: CALL 0x005276d0
0066fb0f: MOV [ESI+0xBF0], EAX

0066fb22: PUSH 0x83c6e8          ; "ChronoDistanceFactor"
0066fb2a: CALL 0x005276d0
0066fb2f: MOV [ESI+0xBF4], EAX

0066fb42: PUSH 0x83c6d8          ; "ChronoTrigger"
0066fb4a: CALL 0x005295f0        ; ReadBool
0066fb4f: MOV [ESI+0xBF8], AL

0066fb61: PUSH 0x83c6c4          ; "ChronoMinimumDelay"
0066fb69: CALL 0x005276d0
0066fb6e: MOV [ESI+0xBFC], EAX

0066fb81: PUSH 0x83c6b0          ; "ChronoRangeMinimum"
0066fb89: CALL 0x005276d0
0066fb8e: MOV [ESI+0xC00], EAX

00670003: PUSH 0x83c464          ; "ChronoHarvTooFarDistance"
0067000b: CALL 0x005276d0
00670010: MOV [ESI+0xD7C], EAX   ; (note: separated from others in the function)
```

### 7.3 Warp Delay Formula

```c
if (ChronoTrigger) {
    delay = distance_leptons / ChronoDistanceFactor;
    if (delay < ChronoMinimumDelay)
        delay = ChronoMinimumDelay;
    if (distance < ChronoRangeMinimum)
        delay = ChronoMinimumDelay;
} else {
    delay = ChronoMinimumDelay;
}
```

**Example delays** (Factor=48, MinDelay=16, at 15fps game speed):
| Distance | Leptons | Delay (frames) | Real time |
|----------|---------|----------------|-----------|
| 1 cell | 256 | 16 (minimum) | ~1.1s |
| 5 cells | 1280 | 26 | ~1.7s |
| 10 cells | 2560 | 53 | ~3.5s |
| 25 cells | 6400 | 133 | ~8.9s |
| 50 cells | 12800 | 267 | ~17.8s |

### 7.4 TechnoClass Chrono Fields

| Offset | Type | Name | Purpose |
|--------|------|------|---------|
| `0x244` | float | WarpFactor | Warp visual progress 0.0 to 1.0 |
| `0x268` | bool | BeingWarpedOut | Warped by external force (Chronosphere) |
| `0x269` | bool | WarpingOut | Actively in warp-out phase |
| `0x27C` | int | ChronoLockRemaining | Frames left in transit |
| `0x280` | CoordStruct (12B) | ChronoDestCoords | Warp destination |
| `0x288` | CoordStruct (12B) | ComputedDestCoords | Resolved by `FUN_00718260` |

### 7.5 Warp Sequence

**Phase 1 — Warp-Out (departure):**
1. `ChronoOutSound` plays
2. `WARPOUT` animation at current position (flat translucent ring)
3. `WarpingOut` = true
4. `WarpFactor` ramps 0.0 -> 1.0 (unit visually fades)
5. `CHRONOSK` sparkle animation plays over unit
6. When WarpFactor reaches 1.0: unit enters limbo (`InLimbo` at `+0x81` = true)
7. `ChronoDestCoords` stores destination

**Phase 2 — In-Transit (chrono lock):**
1. Unit in limbo (removed from map)
2. `ChronoLockRemaining` set by delay formula (see 7.3)
3. Counts down each frame

**Phase 3 — Warp-In (arrival):**
1. Unit placed at `ChronoDestCoords` (unlimboed)
2. `ChronoInSound` plays
3. `WARPIN` animation at destination
4. `CHRONOSK` sparkle plays
5. `WarpingOut` cleared, `WarpFactor` ramps back to 0.0
6. Unit fully materialized

### 7.6 Bridge Handling in Teleport

From decompiled `FUN_00718260` (`MoveTo`):

```c
cell_flags = *(uint *)(cell + 0x140);
if (cell_flags & 0x100) {           // bridge present
    occupants = *(cell + 0xe8);     // bridge layer occupant list
} else {
    occupants = *(cell + 0xe4);     // ground layer occupant list
}

// After resolving position:
if (cell_flags & 0x100) {
    if (!(entity->on_bridge)) {
        entity->on_bridge = 1;          // at +0x8c
        z_position += DAT_00b0ec2c;     // bridge height offset
    }
} else {
    entity->on_bridge = 0;
}
```

---

## 8. Chrono Miner (CMIN) Specifics

### 8.1 CMIN vs War Miner (HARV)

| Aspect | HARV (War Miner) | CMIN (Chrono Miner) |
|--------|------------------|---------------------|
| Locomotor | DriveLocomotionClass | TeleportLocomotionClass + piggybacked Drive |
| Go to ore | Drives | Drives (piggybacked Drive) |
| Return to refinery | Drives | **Teleports** near refinery, then drives last bit |
| Storage | 40 bales | 20 bales |
| Weapon | 20mmRapid turret | None |
| Too-far threshold | 5 cells (HarvesterTooFarDistance) | 50 cells (ChronoHarvTooFarDistance) |

### 8.2 Piggyback Locomotor Mechanism

TeleportLocomotionClass does NOT implement ground movement. It wraps a DriveLocomotionClass via the `IPiggyback` COM interface:

- **Teleport mode**: TeleportLocomotionClass active (warp sequences)
- **Drive mode**: Piggybacked DriveLocomotionClass active (ground movement)

`TechnoClass::AI` (`FUN_004da530`) checks each tick via QueryInterface for IPiggyback. When teleport finishes a warp sequence, it signals completion and the AI swaps the active locomotor.

### 8.3 When CMIN Drives vs Teleports

**Drives (piggybacked DriveLocomotionClass):**
- Moving to ore cells
- Approaching refinery docking position after warp-in
- Docking/unloading at refinery
- Exiting refinery
- When `ChronoLockRemaining` is non-zero
- Short-distance player move commands

**Teleports (TeleportLocomotionClass):**
- Returning to refinery after harvesting
- Long-distance player move commands
- Distance must be within `ChronoHarvTooFarDistance` (50 cells)

**CMIN does NOT teleport TO ore** — only FROM ore back to refinery.

### 8.4 Mission Flow

```
1. SearchOre:
   - Scan for nearby ore (local -> long -> global)
   - CMIN DRIVES to ore (piggybacked Drive locomotor)

2. Harvest:
   - Extract bales from ore cell
   - Storage = 20 bales max

3. Return to Refinery:
   - Find_Nearest_Dock (FUN_004dfcb0)
   - If distance <= ChronoHarvTooFarDistance (50 cells):
     * Chrono warp-out -> transit -> warp-in near refinery
   - Else: drive (or pick closer refinery)

4. Dock at Refinery:
   - After warp-in: DRIVE last few cells to dock pad
   - Switch to UnloadingClass=CMON (empty bay voxel)
   - Accumulator-based unload to house credits

5. Exit and Loop:
   - Undock with facing 0x47 (SE)
   - Return to step 1
```

### 8.5 Sounds & Animations

| Event | Sound/Anim | Asset |
|-------|------------|-------|
| Warp-out | ChronoMinerTeleport + WARPOUT + CHRONOSK | Ring + sparkle at departure |
| Warp-in | ChronoMinerTeleport + WARPIN + CHRONOSK | Ring + sparkle at arrival |

Global rules.ini keys: `WarpIn=WARPIN`, `WarpOut=WARPOUT`, `ChronoSparkle1=CHRONOSK`

---

## 9. Pathfinding Movement Costs

From `FUN_0066d530` file 011 (pathfinding):

| Condition | Cost Factor |
|-----------|-------------|
| Cliff cells (`0x100` flag) | 4.0 base cost |
| Bridge-aware mode | 1000.0 cost |
| Altered passability (`0x40000`) | Multiplier from `DAT_007e37bc` |
| Diagonal moves | Lookup from `DAT_007e3710` / `DAT_007e3730` |

Neighbor expansion: 8 compass directions + tunnel (direction 8) = 9 neighbors max.
Direction offsets from `DAT_007e3774`. Diagonal base costs from `DAT_0081872c`.

---

## 10. Other Locomotion-Related RulesClass Constants

From the `FUN_0066d530` INI parser, in order of struct offset:

| INI Key | RulesClass Offset | Type | Purpose |
|---------|-------------------|------|---------|
| `CloseEnough` | `+0x1718` | lepton | "close enough" distance threshold |
| `Stray` | `+0x171C` | lepton | Max stray distance from guard post |
| `RelaxedStray` | `+0x1720` | lepton | Relaxed stray radius |
| `GuardModeStray` | `+0x1724` | lepton | Guard mode stray radius |
| `TiberiumShortScan` | `+0x1778` | int | Ore scan short radius |
| `TiberiumLongScan` | `+0x177C` | int | Ore scan long radius |
| `SlaveMinerShortScan` | `+0x1780` | int | Slave miner short scan |
| `SlaveMinerSlaveScan` | `+0x1784` | int | Slave miner slave scan |
| `SlaveMinerLongScan` | `+0x1788` | int | Slave miner long scan |
| `SlaveMinerScanCorrection` | `+0x178C` | int | Slave scan correction |
| `SlaveMinerKickFrameDelay` | `+0x1790` | int | Slave kick frame delay |
| `FlightLevel` | `+0x7B4` | int | Default aircraft altitude |
| `ParachuteMaxFallRate` | `+0x7B8` | int | Max parachute fall speed |
| `NoParachuteMaxFallRate` | `+0x7BC` | int | Max no-chute fall speed |
| `HarvesterLoadRate` | `+0x1520` | int | Frames per bale loaded |
| `HarvesterDumpRate` | `+0x1528` | double | Dump rate multiplier |

---

## 11. V3/Dreadnought/Cruise Missile Constants

All parsed from `[General]`, stored on RulesClass:

### V3 Rocket
| INI Key | Offset | Type |
|---------|--------|------|
| `V3RocketPauseFrames` | `+0x4B0` | int |
| `V3RocketTiltFrames` | `+0x4B4` | int |
| `V3RocketPitchInitial` | `+0x4B8` | float |
| `V3RocketPitchFinal` | `+0x4BC` | float |
| `V3RocketTurnRate` | `+0x4C0` | float |
| `V3RocketRaiseRate` | `+0x4C4` | fixed |
| `V3RocketAcceleration` | `+0x4C8` | float |
| `V3RocketAltitude` | `+0x4CC` | int |
| `V3RocketDamage` | `+0x4D0` | int |
| `V3RocketEliteDamage` | `+0x4D4` | int |
| `V3RocketBodyLength` | `+0x4D8` | int |
| `V3RocketLazyCurve` | `+0x4DC` | bool |

### Dreadnought Missile (DMisl)
| INI Key | Offset | Type |
|---------|--------|------|
| `DMislPauseFrames` | `+0x4E4` | int |
| `DMislTiltFrames` | `+0x4E8` | int |
| `DMislPitchInitial` | `+0x4EC` | float |
| `DMislPitchFinal` | `+0x4F0` | float |
| `DMislTurnRate` | `+0x4F4` | float |
| `DMislRaiseRate` | `+0x4F8` | fixed |
| `DMislAcceleration` | `+0x4FC` | float |
| `DMislAltitude` | `+0x500` | int |
| `DMislDamage` | `+0x504` | int |
| `DMislEliteDamage` | `+0x508` | int |
| `DMislBodyLength` | `+0x50C` | int |
| `DMislLazyCurve` | `+0x510` | bool |

### Cruise Missile (CMisl)
| INI Key | Offset | Type |
|---------|--------|------|
| `CMislPauseFrames` | `+0x518` | int |
| `CMislTiltFrames` | `+0x51C` | int |
| `CMislPitchInitial` | `+0x520` | float |
| `CMislPitchFinal` | `+0x524` | float |
| `CMislTurnRate` | `+0x528` | float |
| `CMislRaiseRate` | `+0x52C` | fixed |
| `CMislAcceleration` | `+0x530` | float |
| `CMislAltitude` | `+0x534` | int |
| `CMislDamage` | `+0x538` | int |
| `CMislEliteDamage` | `+0x53C` | int |
| `CMislBodyLength` | `+0x540` | int |
| `CMislLazyCurve` | `+0x544` | bool |

---

## 12. Key Ghidra Function Reference

| Address | Function | System | File |
|---------|----------|--------|------|
| `0x4AF540` | DriveLocomotionClass::Constructor | Drive track init | 034 |
| `0x4B0500` | DriveLocomotionClass::Process | **Main drive tick** — dispatches to Process_Drive_Track or Process_Movement | 034 |
| `0x4B0F20` | DriveLocomotionClass::Process_Drive_Track | Per-step track advancement | 034 |
| `0x4B2630` | DriveLocomotionClass::Process_Movement | Internal helper: picks next cell, starts new track | 034 |
| `0x4CC9A0` | FlyLocomotionClass::Constructor | Aircraft init | 038 |
| `0x4CD600` | FlyLocomotionClass::Process | Main fly tick | 038 |
| `0x4CAE30` | Math::atan2 | Generic trig helper (used across binary, not Fly-specific) | — |
| `0x513C20` | HoverLocomotionClass::Constructor | Hover init | 053 |
| `0x514310` | HoverLocomotionClass::Move | Hover movement | 053 |
| `0x515ED0` | HoverLocomotionClass::SpeedUpdate | Speed calc | 053 |
| `0x54AC40` | JumpjetLocomotionClass::Constructor | Jumpjet init | 061 |
| `0x661EC0` | RocketLocomotionClass::Constructor | Rocket init | 104 |
| `0x69EC50` | ShipLocomotionClass::Constructor | Ship init | 113 |
| `0x6A05F0` | ShipLocomotionClass::Process_Drive_Track | Ship per-step track advancement | 113 |
| `0x6A1C80` | ShipLocomotionClass::Process_Movement | Ship movement helper | 113 |
| `0x718000` | TeleportLocomotionClass::Constructor | Chrono init | 133 |
| `0x718260` | TeleportLocomotionClass::Update_Position | Position update + bridge detection | 133 |
| `0x718B70` | TeleportLocomotionClass::Process | Main chrono tick | 133 |
| `0x75AA90` | WalkLocomotionClass::Constructor | Walk init | 145 |
| `0x728A00` | TunnelLocomotionClass::Constructor | Tunnel init | 136 |
| `0x0066D530` | RulesClass::ReadGeneral | INI parser (all constants) | — |
| `0x4DA530` | FootClass::AI | Per-tick update + locomotor piggyback swap | 040 |
| `0x4DFCB0` | FootClass::Find_Nearest_Dock | Harvester dock find | 040 |
| `0x0055A930` | LocomotionClass::Is_Powered | Trivial getter (`return byte at +0xC`) — **NOT** the gravity-assist function; real gravity-assist source is unidentified | — |
| `0x00578080` | (unnamed) | Ground Z-height lookup | — |
| `0x00565730` | CellClass::GetCell | Cell from coords | — |
| `0x005657A0` | CellClass::GetCellAt | Cell from packed coords | — |

---

## 13. Comparison: Ghidra Findings vs Rust Implementation

### 13.1 Locomotor System — CLSID Mapping

| Locomotor | Ghidra CLSID | Rust Enum | Status |
|-----------|-------------|-----------|--------|
| Drive | `4A582741` | `LocomotorKind::Drive` | MATCH |
| Hover | `4A582742` | `LocomotorKind::Hover` | MATCH |
| Tunnel | `4A582743` | `LocomotorKind::Tunnel` | MATCH |
| Walk | `4A582744` | `LocomotorKind::Walk` | MATCH |
| DropPod | `4A582745` | `LocomotorKind::DropPod` | MATCH |
| Fly | `4A582746` | `LocomotorKind::Fly` | MATCH |
| Teleport | `4A582747` | `LocomotorKind::Teleport` | MATCH |
| Mech | `55D141B8` | `LocomotorKind::Mech` | MATCH |
| Ship | `2BEA74E1` | `LocomotorKind::Ship` | MATCH |
| Jumpjet | `92612C46` | `LocomotorKind::Jumpjet` | MATCH |
| Rocket | `B7B49766` | `LocomotorKind::Rocket` | MATCH |

All 11 CLSIDs match. Unknown CLSIDs default to Teleport in both engines. **Full parity.**

---

### 13.2 Speed & Acceleration

| Aspect | Ghidra (gamemd.exe) | Rust Engine | Status |
|--------|---------------------|-------------|--------|
| Deceleration = 1.5x acceleration | Yes (hardcoded `* 1.5` via `DAT_007e48f0`) | Yes (`DECEL_MULTIPLIER = 4`) | **MISMATCH — Rust uses 4x but original uses 1.5x** |
| Linear speed ramp | `current += accel` | `speed += accel_factor` (SimFixed) | MATCH |
| Slope climb penalty | 9-float lookup table at `DAT_0089ea40` | `slope_climb` SimFixed from INI (default 0.6) | SIMPLIFIED — Rust uses single multiplier, original uses 9-entry table indexed by slope class |
| Slope descend bonus | Multiplier at RulesClass+0x770 | `slope_descend` SimFixed from INI (default 1.2) | SIMPLIFIED — same as above |
| Close-enough threshold | RulesClass+0x1718 (`CloseEnough=`) | `close_enough` SimFixed (default 2.25 cells / 576 leptons) | MATCH — Rust parses from INI |
| Per-unit AccelerationFactor | Per-type field | `accel_factor` SimFixed (default 0.03) | MATCH |
| Per-unit DeaccelerationFactor | Per-type field | `decel_factor` SimFixed (default 0.002) | MATCH |
| SlowdownDistance | Per-type field | `slowdown_distance` i32 (default 500 leptons) | MATCH |
| Fixed-point sim math | Integer/fixed in original | `SimFixed` throughout | MATCH |

**Gap:** Original uses a 9-element slope penalty lookup table per terrain class. Rust uses a single
`slope_climb` / `slope_descend` multiplier. This means slope penalties are uniform across all terrain
types in Rust, whereas the original differentiates (e.g., paved slopes vs dirt slopes).

---

### 13.3 Drive Track System

| Aspect | Ghidra (gamemd.exe) | Rust Engine | Status |
|--------|---------------------|-------------|--------|
| Track table | `DAT_007e7b28`, 12B/entry | `TURN_TRACKS[72]` hardcoded | MATCH — data extracted from original |
| Track waypoints | `DAT_007e7a28`, outer table 16B/entry (pointer+metadata), inner waypoints 12B each (dx, dy, heading) — corrected 2026-05-28: was "12B/entry"; outer stride is 16 | `TrackPoint { x, y, facing }` (12B inner waypoints) | MATCH for inner waypoint struct; Rust `TURN_TRACKS` correctly uses the 12B waypoint layout |
| Track step cost | 7 per step | `TRACK_STEP_COST = 7` | MATCH |
| Track transformation flags | Bit 1=swap XY, Bit 2=negate X, Bit 4=negate Y, Bit 8=cell trigger | flags in `TurnTrack` | NEEDS VERIFICATION — flags exist but transformation logic should be compared |
| Track selection | Turn angle -> table lookup | Turn angle -> `TURN_TRACKS` index | MATCH |
| Movement budget | Subtract 7 per step consumed | `residual` field tracks budget | MATCH |

**Overall: Close parity.** Track data was extracted from the original engine.

---

### 13.4 Teleport / Chrono System

| Aspect | Ghidra (gamemd.exe) | Rust Engine | Status |
|--------|---------------------|-------------|--------|
| Warp phases | WarpOut -> InTransit -> WarpIn | `WarpOut -> Relocate -> WarpIn -> Cooldown` | SIMILAR — Rust adds explicit Cooldown phase |
| WarpFactor (0.0 -> 1.0) visual ramp | Yes, at TechnoClass+0x244 | **NOT IMPLEMENTED** | GAP |
| ChronoLockRemaining (distance delay) | Frames at +0x27C, formula-based | **NOT IMPLEMENTED** — teleport is instant | GAP |
| WARPOUT/WARPIN/CHRONOSK anims | Spawned at departure/arrival | WarpIn/WarpOut AnimRef parsed but **not spawned during teleport** | GAP |
| ChronoInSound/ChronoOutSound | Played at warp events | **NOT IMPLEMENTED** | GAP |
| IPiggyback COM interface | Drive <-> Teleport switching | Piggyback override via `locomotor_override` | PARTIAL — mechanism exists but simpler |
| Fire blocked during warp | Yes | Yes (`combat_fire_gate.rs`) | MATCH |
| Teleporter flag | Per-type flag | `teleporter: bool` on ObjectType | MATCH |
| Bridge detection in MoveTo | cell+0x140 & 0x100 check, Z offset | Bridge layer detection in movement.rs | MATCH in concept |

---

### 13.5 Chrono INI Constants

| INI Key | Ghidra Offset | Rust Field | Status |
|---------|---------------|------------|--------|
| `ChronoDelay` | RulesClass+0xBEC | **NOT PARSED** | GAP |
| `ChronoReinfDelay` | +0xBF0 | **NOT PARSED** | GAP |
| `ChronoDistanceFactor` | +0xBF4 (default 48) | **NOT PARSED** | GAP |
| `ChronoTrigger` | +0xBF8 (bool) | **NOT PARSED** | GAP |
| `ChronoMinimumDelay` | +0xBFC (default 16) | **NOT PARSED** | GAP |
| `ChronoRangeMinimum` | +0xC00 | **NOT PARSED** | GAP |
| `ChronoHarvTooFarDistance` | +0xD7C (default 50) | `chrono_harv_too_far_distance: i32` | MATCH |
| `WarpIn` | (animation name) | `warp_in: AnimRef` | MATCH |
| `WarpOut` | (animation name) | `warp_out: AnimRef` | MATCH |
| `WarpAway` | (animation name) | `warp_away: AnimRef` | MATCH |

**6 of 10 chrono constants are not parsed.** The warp delay formula cannot work without them.

---

### 13.6 Harvester / Miner System

| Aspect | Ghidra (gamemd.exe) | Rust Engine | Status |
|--------|---------------------|-------------|--------|
| CMIN drives to ore, teleports back | Yes | Yes (MinerKind::Chrono) | MATCH |
| CMIN never teleports TO ore | Correct | Correct | MATCH |
| War Miner capacity = 40 | Yes | `war_miner_capacity = 40` | MATCH |
| Chrono Miner capacity = 20 | Yes | `chrono_miner_capacity = 20` | MATCH |
| HarvesterTooFarDistance | 5 cells | `harvester_too_far_distance = 5` | MATCH |
| ChronoHarvTooFarDistance | 50 cells | `chrono_harv_too_far_distance = 50` | MATCH |
| TiberiumShortScan | +0x1778 | `tiberium_short_scan = 6` | MATCH |
| TiberiumLongScan | +0x177C | `tiberium_long_scan = 48` | MATCH |
| SlaveMiner scan constants | 5 fields at +0x1780..+0x1790 | All 5 parsed | MATCH |
| HarvesterLoadRate | +0x1520 (from INI) | Hardcoded `harvest_tick_interval = 37` | HARDCODED — not from INI |
| HarvesterDumpRate | +0x1528 (from INI) | Hardcoded `unload_tick_interval = 14` | HARDCODED — not from INI |
| Undock facing 0x47 (SE) | Yes | In refinery dock state machine | MATCH |
| Mission flow (Search->Harvest->Return->Dock->Unload) | Yes | `MinerState` FSM matches | MATCH |

---

### 13.7 Air Movement

| Aspect | Ghidra (gamemd.exe) | Rust Engine | Status |
|--------|---------------------|-------------|--------|
| FlightLevel | RulesClass+0x7B4 (from INI) | Hardcoded `FLY_CRUISE_ALTITUDE = 600` | HARDCODED — not from INI |
| Fly accel formula | `delta = max_speed / (accel_factor * 60)` | Custom accel in air_movement.rs | DIFFERS — Rust uses per-unit AccelerationFactor, original divides by 60 |
| Fly deceleration | Symmetric with accel | `FLY_CLIMB_RATE = 300` lep/s | SIMPLIFIED |
| Gravity-assist (downhill +1/3 speed) | Yes (`FUN_0055a930`) | **NOT IMPLEMENTED** | GAP |
| Descent slowdown zones (20/50 ticks) | Yes | Not found | GAP |
| JumpJet params (9 INI keys) | Per-type fields | All 9 parsed into `JumpjetParams` | MATCH |
| Jumpjet decel = 1.5x accel | Yes | `SIM_1_5` constant | MATCH |
| ParachuteMaxFallRate | +0x7B8 | **NOT PARSED** | GAP |
| NoParachuteMaxFallRate | +0x7BC | **NOT PARSED** | GAP |

---

### 13.8 Tunnel Movement

| Aspect | Ghidra (gamemd.exe) | Rust Engine | Status |
|--------|---------------------|-------------|--------|
| TunnelSpeed from INI | Yes | `tunnel_speed: SimFixed` (default 6.0) | MATCH |
| Burrow threshold | Distance-based | `BURROW_THRESHOLD_CELLS = 11` | MATCH (approximate) |
| Dig in/out phases | Yes | `DigIn` (0.8s) / `DigOut` (0.8s) | MATCH |
| Underground straight-line travel | Yes | Yes | MATCH |

---

### 13.9 Rocket / Missile Constants

| INI Key Group | Ghidra Offsets | Rust Engine | Status |
|---------------|---------------|-------------|--------|
| V3Rocket (12 keys) | +0x4B0..+0x4DC | **NOT PARSED** | GAP |
| DMisl (12 keys) | +0x4E4..+0x514 | **NOT PARSED** | GAP |
| CMisl (12 keys) | +0x518..+0x548 | **NOT PARSED** | GAP |
| Generic rocket movement | — | `RocketPhase` FSM with hardcoded constants | PARTIAL — phases exist but no INI-driven params |

**36 missile/rocket INI keys are not parsed.** Rocket movement uses hardcoded constants:
- `ASCEND_FRACTION = 0.4`
- `PEAK_ALTITUDE = 400`
- `LAUNCH_DURATION_S = 0.3`

---

### 13.10 Pathfinding / Movement Costs

| Aspect | Ghidra (gamemd.exe) | Rust Engine | Status |
|--------|---------------------|-------------|--------|
| Per-SpeedType terrain costs | Yes, from terrain INI sections | `TerrainCostGrid` + `terrain_rules.rs` | MATCH |
| 7 SpeedType modifiers per terrain | Yes | All 7 parsed (Foot, Track, Wheel, Float, Amphibious, FloatBeach, Hover) | MATCH |
| Cliff cost multiplier (4.0) | Yes | `CLIFF_HEIGHT_THRESHOLD = 3 levels` | SIMILAR — different mechanism |
| Bridge-aware pathing | 1000.0 cost for bridge mode | `LayeredPathGrid` with separate bridge layer | DIFFERENT — Rust uses layer separation, original uses cost inflation |
| Diagonal base costs | Lookup from `DAT_007e3710` | Built into A* step cost | IMPLICIT |
| Road bonus | Terrain-dependent | `COST_ROAD = 120` (20% bonus) | MATCH |
| Stray / GuardModeStray | +0x171C / +0x1724 | **NOT PARSED** | GAP |
| RelaxedStray | +0x1720 | **NOT PARSED** | GAP |

---

### 13.11 Bridge Handling

| Aspect | Ghidra (gamemd.exe) | Rust Engine | Status |
|--------|---------------------|-------------|--------|
| Bridge flag in cell | `cell+0x140 & 0x100` | `has_bridge_deck()` on PathGrid | MATCH (concept) |
| Separate bridge/ground occupant lists | `+0xe4` (ground), `+0xe8` (bridge) | `OccupancyMap` + `occupied_bridge` | MATCH |
| Bridge Z offset | `DAT_00b0ec2c` added to Z | `bridge_deck_level()` with visual offset | MATCH |
| On-bridge flag | entity+0x8c | `BridgeOccupancy` component | MATCH |
| Preemptive bridge detection | Not found | Yes (prevents render flicker) | RUST EXTRA |

---

## 14. Summary Scorecard

| Category | Items Checked | Full Match | Simplified/Partial | Gap/Missing |
|----------|--------------|------------|---------------------|-------------|
| Locomotor CLSIDs | 11 | 11 | 0 | 0 |
| Speed/Acceleration | 9 | 7 | 2 (slope tables) | 0 |
| Drive Tracks | 6 | 5 | 1 (flags) | 0 |
| Teleport/Chrono | 10 | 3 | 1 | 6 |
| Chrono INI Constants | 10 | 4 | 0 | 6 |
| Harvester/Miner | 12 | 10 | 0 | 2 (load/dump rates) |
| Air Movement | 9 | 3 | 2 | 4 |
| Tunnel Movement | 4 | 4 | 0 | 0 |
| Rocket/Missile Constants | 36 | 0 | 0 | 36 |
| Pathfinding/Costs | 8 | 4 | 2 | 2 |
| Bridge Handling | 5 | 4 | 0 | 0 (+1 extra) |
| **TOTAL** | **120** | **55 (46%)** | **8 (7%)** | **56 (47%)** |

---

## 15. Priority Gaps to Close

### High Priority (affects gameplay correctness)

1. **Chrono warp delay formula** — Parse `ChronoDistanceFactor`, `ChronoMinimumDelay`,
   `ChronoRangeMinimum`, `ChronoTrigger` from `[General]`. Implement distance-based
   `ChronoLockRemaining` instead of instant teleport. Without this, chrono miners and
   chrono legionnaires behave incorrectly (instant vs multi-second warp).

2. **WarpFactor visual ramp** — Implement 0.0->1.0->0.0 fade during warp phases.
   Currently units snap-teleport with no visual transition.

3. **HarvesterLoadRate / HarvesterDumpRate from INI** — Currently hardcoded at 37/14 ticks.
   Should parse from `[General]` to match mod-configurable behavior.

4. **FlightLevel from INI** — Currently hardcoded to 600. Should parse from `[General]`
   (RulesClass+0x7B4). Affects aircraft cruising altitude.

### Medium Priority (affects visual/audio fidelity)

5. **WARPOUT / WARPIN / CHRONOSK animation spawning** — AnimRefs are parsed but never
   actually spawned during teleport sequences.

6. **ChronoInSound / ChronoOutSound** — No audio during chrono warp events.

7. **Fly aircraft gravity-assist** — Up to +33% speed bonus when flying downhill. Missing.

8. **Stray / RelaxedStray / GuardModeStray** — Guard behavior radius not implemented.

### Low Priority (completeness / future features)

9. **V3/DMisl/CMisl rocket constants** (36 keys) — Rocket movement uses hardcoded
   values instead of INI-driven parameters. Blocks faithful V3/Dreadnought behavior.

10. **9-element slope penalty table** — Current single-value slope multiplier is a
    simplification. Original indexes by terrain slope class.

11. **Fly deceleration zones** (20/50 tick approach slowdown) — Not implemented.

12. **Parachute fall rate constants** — Not parsed from INI.
