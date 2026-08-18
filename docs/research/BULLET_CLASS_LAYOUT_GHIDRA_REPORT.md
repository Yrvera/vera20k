# BulletClass Runtime Instance Struct Layout

**Program:** gamemd.exe
**Allocation size:** 0x160 (352 bytes) — via `operator_new(0x160)` in COM factory at 0x006C5086
**Confidence:** High (verified from binary: constructor, AI, init, detonation, bounce functions)

## Inheritance Chain

```
IUnknown                  (COM interface, provides vtable[0])
  └─ IPersistStream       (vtable[1])
      └─ IRTTITypeInfo     (vtable[2])
          └─ INoticeSink   (vtable[3])
              └─ AbstractClass    (0x00-0x23)
                  └─ ObjectClass  (0x00-0xAB, size = 0xAC = 172 bytes)
                      └─ BulletClass (0xAC-0x15F, adds 180 bytes)
```

## Complete Field Map

### AbstractClass / INoticeSink Base (0x00 - 0x23)

| Offset | Size | Type | Name | Init Value | Notes |
|--------|------|------|------|------------|-------|
| 0x00 | 4 | ptr | vtable | vtable__BulletClass (0x7E46E4) | Primary vtable |
| 0x04 | 4 | ptr | vtable_IRTTITypeInfo | vtable__BulletClass__secondary_4 (0x7E46C8) | IRTTITypeInfo (NOT IPersistStream — IPersistStream methods live in primary vtable slots +0x00..+0x1C) |
| 0x08 | 4 | ptr | vtable_INoticeSink | vtable__BulletClass__secondary_8 (0x7E46C0) | |
| 0x0C | 4 | ptr | vtable_INoticeSource | vtable__BulletClass__secondary_12 (0x7E46B8) | |
| 0x10 | 4 | int | UniqueID | — | Unique instance ID |
| 0x14 | 1 | byte | AbstractFlags | — | bit 0 = IsTechno (verified in Select/UnInit/Teleport `(byte @+0x14) & 1`). Bits 1–2 purpose not binary-verified here — historical doc claims "IsOnMap / IsNetPlayer" were not confirmed. |
| 0x18 | 4 | int | unknown_0x18 | — | (padding / alignment) |
| 0x1C | 4 | int | RefCount | 0 | COM reference count |
| 0x20 | 1 | bool | Dirty | 0 | Needs save/sync |
| 0x21-0x23 | 3 | — | padding | — | |

### ObjectClass Fields (0x24 - 0xAB)

| Offset | Size | Type | Name | Init Value | Notes |
|--------|------|------|------|------------|-------|
| 0x24 | 4 | int | unknown_0x24 | 0 | |
| 0x28 | 4 | int | unknown_0x28 | 0 | |
| 0x2C | 4 | int | FallRate | 0 | Rate of falling |
| 0x30 | 4 | ptr | NextObject | 0 | Linked list of objects in cell |
| 0x34 | 4 | ptr | AttachedTag | 0 | Trigger tag |
| 0x38 | 4 | ptr | AttachedBomb | 0 | IvanBomb / C4 |
| 0x3C-0x63 | 40 | — | unknown | — | (Timer objects, audio state) |
| 0x64 | 4 | int | CustomSound | -1 | Sound index override |
| 0x68 | 1 | bool | BombVisible | 0 | |
| 0x69-0x6B | 3 | — | padding | — | |
| 0x6C | 4 | int | Health | 0xFF | Current health (bullets use this for Damage) |
| 0x70 | 4 | int | EstimatedHealth | 0xFF | Pre-impact health estimate |
| 0x74 | 4 | int | unknown_0x74 | — | |
| 0x78 | 4 | int | Layer | 1 | Map layer / height level |
| 0x7C-0x7F | 4 | — | unknown | — | |
| 0x80 | 1 | bool | NeedsRedraw | 0 | |
| 0x81 | 1 | bool | InLimbo | 1 | True when not placed on map |
| 0x82 | 1 | bool | InOpenToppedTransport | 0 | |
| 0x83 | 1 | bool | IsSelected | 0 | |
| 0x84 | 1 | bool | HasParachute | 0 | |
| 0x85-0x87 | 3 | — | padding | — | |
| 0x88 | 4 | ptr | Parachute | 0 | AnimClass for parachute |
| 0x8C | 1 | bool | OnBridge | 0 | On bridge surface |
| 0x8D | 1 | bool | IsFallingDown | 0 | Gravity-affected fall state; for bullets: "HasDropped" flag |
| 0x8E | 1 | bool | WasFallingDown | 0 | |
| 0x8F | 1 | bool | IsABomb | 0 | |
| 0x90 | 1 | bool | IsAlive | 1 | False = destroyed, AI skips |
| 0x91-0x93 | 3 | — | padding | — | |
| 0x94 | 4 | int | LastLayer | -1 | Previous map layer |
| 0x98 | 1 | bool | IsInLogic | 0 | |
| 0x99 | 1 | bool | IsVisible | 1 | |
| 0x9A-0x9B | 2 | — | padding | — | |
| 0x9C | 4 | int | Location.X | 0 (sentinel) | World position X in leptons |
| 0xA0 | 4 | int | Location.Y | 0 (sentinel) | World position Y in leptons |
| 0xA4 | 4 | int | Location.Z | 0 (sentinel) | World position Z in leptons |
| 0xA8 | 4 | ptr | LineTrailer | 0 | Line trail pointer |

### BulletClass-Specific Fields (0xAC - 0x15F)

| Offset | Size | Type | Name | Init Value | Notes |
|--------|------|------|------|------------|-------|
| 0xAC | 4 | ptr | Type | 0 | -> BulletTypeClass |
| 0xB0 | 4 | ptr | Owner | 0 | -> TechnoClass (firer/source) |
| 0xB4 | 1 | bool | IsNetPlayerOwned | 0 | Set to 1 if firer is player-controlled + has spotter; skips pre-impact damage |
| 0xB5-0xB7 | 3 | — | padding | — | |
| **0xB8-0xDF** | **40** | **struct** | **ProximityDetector** | — | **Embedded proximity/approach sub-object (initialized by FUN_004E1100)** |
| 0xB8 | 4 | int | Prox.StartFrame | g_CurrentFrame | Frame when bullet was created |
| 0xBC | 4 | int | Prox.unknown_04 | — | |
| 0xC0 | 4 | int | Prox.unknown_08 | 0 | |
| 0xC4 | 4 | int | Prox.TimestampFrame | g_CurrentFrame | Another frame timestamp |
| 0xC8 | 4 | int | Prox.unknown_10 | — | |
| 0xCC | 4 | int | Prox.ArmingDelay | 0 | Ticks before proximity activates |
| 0xD0 | 4 | int | Prox.ReferenceX | 0 | Reference coord X for distance calc |
| 0xD4 | 4 | int | Prox.ReferenceY | 0 | Reference coord Y |
| 0xD8 | 4 | int | Prox.ReferenceZ | 0 | Reference coord Z |
| 0xDC | 4 | int | Prox.ClosestDistance | 0 | Closest distance watermark |
| 0xE0 | 1 | bool | Bright | 0 | Bright draw flag (from weapon) |
| 0xE1-0xE7 | 7 | — | padding | — | |
| 0xE8 | 8 | double | Velocity.X | 0.0 | Leptons per tick, horizontal X |
| 0xF0 | 8 | double | Velocity.Y | 0.0 | Leptons per tick, horizontal Y |
| 0xF8 | 8 | double | Velocity.Z | 0.0 | Leptons per tick, vertical |
| 0x100 | 4 | int | unknown_0x100 | 0 | Possibly padding after velocity doubles; unused in AI |
| 0x104 | 1 | bool | IsActive | 1 | Active flag (not same as IsAlive) |
| 0x105 | 1 | bool | IsCourseLocked | 1 | True while initial heading is locked (homing missiles) |
| 0x106-0x107 | 2 | — | padding | — | |
| 0x108 | 4 | int | CourseLockCounter | 0 | Ticks since launch, compared to BulletTypeClass.CourseLockDuration |
| 0x10C | 4 | ptr | Target | 0 | -> AbstractClass (current tracked target; retargets to cell if target dies) |
| 0x110 | 4 | int | TargetSpeed | 0 | Desired speed in leptons/tick; set from WeaponType.Speed |
| 0x114 | 4 | int | HouseColorIndex | -1 | Color palette index from firer's house (if FirersPalette=yes), else -1 |
| 0x118 | 4 | int | ApproachSampleCount | 0 | Sample count for approach-rate averaging; counts up to 60 |
| 0x11C | 4 | — | padding | — | Aligns ApproachSum to 8 |
| 0x120 | 8 | double | ApproachSum | 0.0 | Running approach-rate accumulator; exponential moving average after 60 samples |
| 0x128 | 4 | ptr | WH | 0 | -> WarheadTypeClass |
| 0x12C | 1 | byte | AnimFrame | 0 | Current sprite frame index; wraps between AnimLow and AnimHigh |
| 0x12D | 1 | byte | AnimTimer | 0 | Ticks until next frame; decremented each tick, reset to AnimRate |
| 0x12E-0x12F | 2 | — | padding | — | |
| 0x130 | 4 | ptr | WeaponType | 0 | -> WeaponTypeClass (set by FUN_0046B260 after creation) |
| 0x134 | 4 | int | SourceCoord.X | 0 | Firing origin X (leptons) |
| 0x138 | 4 | int | SourceCoord.Y | 0 | Firing origin Y (leptons) |
| 0x13C | 4 | int | SourceCoord.Z | 0 | Firing origin Z (leptons) |
| 0x140 | 4 | int | TargetCoord.X | 0 | Original target X (leptons); set during Fire_At |
| 0x144 | 4 | int | TargetCoord.Y | 0 | Original target Y (leptons) |
| 0x148 | 4 | int | TargetCoord.Z | 0 | Original target Z (leptons) |
| 0x14C | 2 | short | LastCell.X | -1 (0xFFFF) | Last cell coord X (cell units); updated at end of AI |
| 0x14E | 2 | short | LastCell.Y | -1 (0xFFFF) | Last cell coord Y (cell units) |
| 0x150 | 4 | int | RockerScale | 0x100 | DirectRocker force scale, Q8.8 fixed-point (0x100 = 1.0×). Set to 0x100 by `BulletClass::Init` at `0x004664C0`; no other writers. Read in `WarheadTypeClass::Detonate` at `0x004697FC` (DirectRocker branch): `force = (RockerScale × Damage) >> 8 × Rules+0x18b4 / const`, capped at 4.0, then passed to `target.vtbl+0x3D8`. See `BULLETCLASS_LIFECYCLE_AND_TIER1_VERIFICATIONS_GHIDRA_REPORT.md` §1.1. |
| 0x154 | 4 | ptr | BounceAnim | 0 | -> AnimClass for impact/bounce animation |
| 0x158 | 1 | bool | IsWaitingForAnim | 0 | True = bullet is in limbo waiting for BounceAnim to finish before destroying |
| 0x159-0x15F | 7 | — | padding/unused | — | To 0x160 total size |

## Key Relationships

### BulletClass -> BulletTypeClass (offset 0xAC)

The BulletTypeClass pointer provides all immutable properties read from INI.
Access pattern: `*(BulletTypeClass + offset)` with direct byte offsets.
See existing report for BulletTypeClass layout (0x294-0x2F7).

### BulletClass -> Owner (offset 0xB0)

Points to the TechnoClass that fired the bullet. Used for:
- Alliance checks via `HouseClass::Is_Ally`
- Getting the firer's house color (offset 0x114)
- Determining if pre-impact damage should be applied

### BulletClass -> Target (offset 0x10C)

Points to the AbstractClass the bullet is tracking. This is NOT always the same
as what was originally shot at:
- If the target dies, it's retargetted to the CellClass at the target's last position
  (see FUN_00468430/FUN_00468480)
- If the target is not on the map, it can be set to null
- The homing code (ROT > 0) uses `Target->GetCoords()` (vtable+0x58) to track

### BulletClass -> WarheadTypeClass (offset 0x128)

Points to the WarheadType used for detonation. Passed directly from the WeaponType
during Fire_At. Used in `FUN_00468D80` (detonation) and `WarheadTypeClass::Detonate`.

### BulletClass -> WeaponTypeClass (offset 0x130)

Set after construction via `FUN_0046B260`. Read via `FUN_0046B270`. Used for
determining weapon properties during detonation and shrapnel spawning.

## Velocity Vector (0xE8 - 0xFF)

Three IEEE 754 doubles, 8 bytes each:

```
+0xE8: Velocity.X (double) — leptons/tick, screen-horizontal (east+)
+0xF0: Velocity.Y (double) — leptons/tick, screen-horizontal (south+)
+0xF8: Velocity.Z (double) — leptons/tick, vertical (up+)
```

The velocity vector is:
- Set during `TechnoClass::Fire_At` based on heading, elevation angle, and speed
- Modified each tick by the movement system (gravity for arcing, homing rotation for ROT>0)
- Its magnitude represents the bullet's current speed in leptons/tick
- When velocity is completely zero, a default of VelX=100.0 is forced (prevents stuck bullets)

## ProximityDetector Sub-Object (0xB8 - 0xDF)

Embedded timer/detector object initialized by `FUN_004E1100` and queried by
`FUN_004E11F0`. Returns proximity status:

- **0**: Not close enough or arming delay not expired -- continue flying
- **1**: Within 32 leptons of reference (half-distance metric) -- very close, detonate
- **2**: Within 256 leptons AND distance is increasing -- overshot, detonate

The reference coordinate (0xD0-0xD8) is set to the target's position when the
proximity check starts. The closest-distance watermark (0xDC) tracks the minimum
half-distance seen, detecting when the bullet starts moving away (overshoot).

## Constructor Call Chain

1. `operator_new(0x160)` -- allocates 352 bytes
2. `BulletClass::Constructor` at 0x00466380 (ECX = this):
   a. Calls `ObjectClass::Constructor` at 0x005F3900
      - Calls `INoticeSink::Constructor` at 0x00410170
      - Sets ObjectClass fields (location, health, flags)
      - Registers in global object arrays
   b. Sets BulletClass-specific fields to zero/defaults
   c. Initializes ProximityDetector sub-object at this+0xB8 via `FUN_004E1100`
   d. Writes BulletClass vtable pointers
   e. Calls `FUN_00410230(this+0x4)` to set up network ID
   f. Registers in BulletClass global array (DAT_00A8ED40)
3. `FUN_004664c0` (thiscall, post-construction init):
   - Sets Type, Owner, Target, WarheadType, Damage, TargetSpeed, Bright
   - Initializes AnimTimer from BulletTypeClass.AnimRate
   - Sets HouseColorIndex from firer's house if FirersPalette=yes

## Decompilation Source Functions

| Address | Name | Role |
|---------|------|------|
| 0x006C5086 | COM ClassFactory::CreateInstance | operator_new(0x160) + constructor |
| 0x00466380 | BulletClass::Constructor | Field initialization |
| 0x00466560 | BulletClass::Destructor | Cleanup |
| 0x004664C0 | BulletClass::PostInit | Sets Type, Owner, Target, WH, Speed, Bright |
| 0x004666E0 | BulletClass::AI | Main per-tick update (767 decompiled lines) |
| 0x00468D80 | BulletClass::Detonate | Warhead detonation logic |
| 0x00468BB0 | BulletClass::BounceCheck | Deflection / bounce conditions |
| 0x0046B260 | BulletClass::SetWeapon | Sets WeaponType at 0x130 |
| 0x0046B270 | BulletClass::GetWeapon | Reads WeaponType at 0x130 |
| 0x0046B310 | BulletClass::SpawnShrapnel | Creates sub-bullets from existing |
| 0x0046B050 | BulletClass::Create | COM CoCreateInstance wrapper |
| 0x004E1100 | ProximityDetector::Init | Initializes embedded detector |
| 0x004E11F0 | ProximityDetector::Check | Returns 0/1/2 proximity status |
| 0x00468430 | BulletClass::UpdateTarget | Retargets if target died |

## Cross-Reference with BulletTypeClass

Key BulletTypeClass fields accessed from BulletClass::AI (direct byte offsets from
BulletTypeClass pointer at this+0xAC):

| BT Offset | INI Key | Accessed For |
|-----------|---------|--------------|
| 0x294 | Airburst | Skip cluster detonation; affects homing proximity |
| 0x295 | Floater | Use alternate gravity function |
| 0x296 | SubjectToCliffs | Bounce check: cliff collision |
| 0x298 | SubjectToWalls | Bounce check: wall collision |
| 0x299 | VeryHigh | Approach-rate detonation exemption |
| 0x29B | Arcing | Select ballistic trajectory mode |
| 0x29C | Dropping | Enable "dropping" behavior (e.g. paratroop bombs) |
| 0x29D | Level | Bounce check: ground-level movement |
| 0x2A0 | Ranged | Enable proximity detector for ROT<=0 |
| 0x2A2 | Inaccurate | Skip target-snap on detonation |
| 0x2A3 | FlakScatter | Burst below target altitude |
| 0x2A4 | AA | Anti-air bounce check |
| 0x2A6 | Degenerates | Decrement Health (damage) if > 5, each tick |
| 0x2A7 | Bouncy | Enable velocity reflection on ground hit |
| 0x2A9 | FirersPalette | Use firer's house color palette |
| 0x2AC | Cluster | Sub-munition count for detonation |
| 0x2C0 | Vertical | Straight vertical descent mode |
| 0x2C8 | Elasticity | Bounce energy retention factor (double) |
| 0x2D0 | Acceleration | Speed change per tick (homing) |
| 0x2D8 | Trailer | AnimTypeClass* for trail effect |
| 0x2DC | ROT | Rate of turn; <=0 = ballistic/straight, >0 = homing |
| 0x2E0 | CourseLockDuration | Ticks of locked heading after launch |
| 0x2E4 | SpawnDelay | Trailer spawn interval (ticks) |
| 0x2E8 | (uninit by ReadINI) | constructor-zeroed; no INI writer; AI's "max" trailer-cadence branch is permanently dead. Previously labeled `TrailerSeperation` — that key actually lives on AnimType, not BulletType. See `BULLETTYPECLASS_GHIDRA_REPORT.md` §5. |
| 0x2F4 | AnimLow | First animation frame |
| 0x2F5 | AnimHigh | Last animation frame |
| 0x2F6 | AnimRate | Ticks per animation frame |

## Notes

- `param_1` in BulletClass::AI is typed as `int *`, meaning all `param_1[N]` accesses
  represent byte offset `N * 4`. This is critical for correct offset computation.
- The ObjectClass struct is 172 bytes (0xAC). BulletClass adds 180 bytes (0xB4) for
  a total of 352 bytes (0x160).
- The ProximityDetector is an embedded sub-object (not a pointer), occupying 40 bytes
  at offset 0xB8. It has its own initialization and query functions.
- Offset 0x100 has uncertain purpose. 0x100 is initialized to 0 and not visibly
  accessed in AI.
- Offset 0x150 = **RockerScale** (DirectRocker force scale, Q8.8 fixed-point, default
  0x100 = 1.0×). Read in `WarheadTypeClass::Detonate` at `0x004697FC`. See
  `BULLETCLASS_LIFECYCLE_AND_TIER1_VERIFICATIONS_GHIDRA_REPORT.md` §1.1.
- The bullet's "damage" is stored in ObjectClass.Health (offset 0x6C). When
  Degenerates=yes, this value decrements each tick (minimum 5).

---

## Tier 7 application record (2026-08-17, Claude Code session)

Corridor: `docs/plans/2026-08-17-ghidra-typing-corridor-program.md` row 7, "Weapon-fire
corridor". Snapshot before mutations:
`<local>/Documents/ghidra-backups/2026-08-17-pre-tier7` (17 files, 243,359,753 bytes,
verified with the program closed).

**A `/BulletClass` struct now exists — 352 bytes (0x160), 24 named fields.** There was none
before this tier. The size is proven twice and survived adversarial review at the byte level:
the allocation site `PUSH 0x160; CALL operator new; MOV ECX,EAX; CALL 0x00466380` at
0x006C50BC-0x006C50CF (the sole xref to the constructor), and the constant `MOV EAX,0x160; RET`
at 0x0046B540. Caveat recorded: that second address has no defined function in Ghidra and its
vtable slot was never proved, so "the Size_Of virtual" is an unproven identity — the size does
not depend on it.

### This doc was RIGHT and the fresh investigation was wrong — 0x150

Worth recording as a method point, because the usual failure runs the other way. A fresh
investigator proposed `DamageMultiplier` for 0x150 from the arithmetic alone; a critic correctly
refused to confirm the consumer. This doc already said **RockerScale**, with a citation. The
binary settles it at 0x004697FC:
`MOV EAX,[ESI+0x150]; MOV ECX,[0x008871E0]; IMUL EAX,[ESI+0x6C]; SAR EAX,0x8; MOV [ESP+0x1C],EAX;
FILD [ESP+0x1C]; FMUL float ptr [ECX+0x18B4]`. So the Q8.8 arithmetic `(scale * damage) >> 8` is
real, but the product is a **rocking impulse**, not damage — it is multiplied by a RulesClass
rocking coefficient. `RockerScale` is the correct name; `DamageMultiplier` would have misled
every future reader. Applied as `RockerScale`. Research docs are leads, but a lead with a cited
consumer beats a fresh derivation without one.

### Three facts a port needs

1. **Velocity at +0xE8 is three 8-byte doubles.** Survived attack at the byte level:
   0x00468992 / 0x004689A5 / 0x004689B8 are `DD 83 E8/F0/F8 000000` — `DD /0` is `FLD m64fp`;
   a 4-byte float would encode `D9 /0`. Unlimbo copies exactly 24 bytes
   (0x00468694 `LEA EDI,[EBX+0xE8]`, `MOV ECX,6`, 0x004686A0 `REP MOVSD`). The shape-frame
   virtual at 0x00468016 feeds [ESI+0xEC]:[ESI+0xE8] into `atan2`, so **a bullet's facing is
   derived from floating-point velocity every frame** — a deterministic-lockstep hazard in the
   original that fixed-point elsewhere does not hide.
2. **No dedicated Damage field** — Init stores the weapon damage into the inherited
   ObjectClass::Health slot at +0x6C (0x004664F6), consistent with this doc's existing note.
   UNSETTLED: the detonation-side read was not located this session.
3. **No owning-house pointer was observed** in 0xAC-0x15F; the house is reached through the
   firer (Init 0x00466527-0x00466533: `MOV EAX,[ESI+0x21C]; MOV EAX,[EAX+0x16054]`). This is
   deliberately NOT recorded as "proven absent" — a critic downgraded that claim, and 0xB4-0xCF
   plus four smaller gaps remain uncharacterized. What stands regardless: when the firer dies
   first, `PointerExpired` nulls +0xB0 and the attacking house becomes 0, so kill attribution is
   lost by construction — native behaviour, not a bug.

### Fire-path prototypes applied (6, all `__thiscall` with `TechnoClass *` receivers)

Every arity taken from the callee's own RET immediate and re-enumerated by a critic; **no
function in the corridor has mixed RET immediates**, and all six read incoming ECX at entry.

| Address | Slot | Prototype | RET sites |
|---|---|---|---|
| 0x006FC0B0 `GetFireError` | +0x3C0 (240) | `int(this, void* target, int weaponIdx, int checkCanFire)` | 32, all `RET 0xC` |
| 0x006FCDB0 `Assign_Target` | +0x3C8 (242) | `void(this, void* target)` | 1, `RET 0x4` |
| 0x006FDD50 `FireAtSpawnsBullet` | +0x3CC (243) | `BulletClass*(this, void* target, int weaponIdx)` | 5, all `RET 0x8` |
| 0x006F3330 `SelectWeaponAgainst` | +0x2E4 (185) | `int(this, void* target)` | 16, all `RET 0x4` |
| 0x0070E140 `GetWeapon` | +0x3F8 (254) | `void*(this, int weaponIdx)` | 2, all `RET 0x4` |
| 0x006F3970 `GetWeaponRange` | non-virtual | `int(this, int weaponIdx)` | 5, all `RET 0x4` |

**`GetFireError`: Ghidra's decompiler drops two of its three arguments.** It renders
`char __thiscall(TechnoClass*, int*)`. Three independent lines say otherwise and a critic
confirmed all three: `RET 0xC` at all 32 exits; the callee frame math (`SUB ESP,0x10; PUSH EBX`,
then 0x006FC0B4 `MOV EBX,[ESP+0x18]` resolving to entry+4 and immediately `TEST`ed as a
standalone pointer, ruling out a by-value struct); and two callers pushing three separate dwords
— `BuildingClass__GetFireError` at 0x00447FC3-0x00447FD2 and the AircraftClass override at
0x0041A9F1-0x0041A9F4. It also returns a full `int`: 30 constant exits all encode `B8 imm32`
(`MOV EAX, imm32`), never `B0` (`MOV AL, imm8`). Anyone porting from the rendered signature would
silently lose the weapon index and the check flag. The full return enum is in the plate comment
at 0x006FC0B0.

### Holes — recorded, not guessed

| Offset | State |
|---|---|
| 0x120 | Applied as `undefined8`, NOT `double`. The 8-byte extent is supported by the dword zero-stores at 0x004663AA (+0x120) and 0x004663E9 (+0x124), but no qword FP access to it was ever located, so a `double` typing would be a guess. An investigator citation naming 0x004663EF as its upper half was wrong — that address stores to +0x128, the Warhead field. |
| 0xB4 | byte; writer proven, gates the estimated-health prediction subtract; role beyond that unproven |
| 0xE0 | byte from WeaponTypeClass+0x12F; one consumer located (scorch/crater emission) |
| 0x100 | dword, zeroed at 0x004663C5, no reader found |
| 0x104 | byte set to 1 at 0x004663B0, no reader found |
| 0x114 | writer, source (`HouseClass+0x16054`) and the `FirersPalette` gate all proven; consumer not located |

Uncharacterized ranges left without fields: 0xB4-0xCF (28 bytes, includes the 40-byte tracker
embedded at 0xB8), 0x11C-0x11F, 0x12E-0x12F, 0x14C-0x14F.

Storage the constructor leaves uninitialised (matters for snapshots and hashes): 0xE8-0xFF
(velocity — written only by Unlimbo), 0x150 (written only by Init), and 0xBC / 0xC8 inside the
0xB8 tracker.

### Residuals

- **InfantryClass fire-slot binding UNPROVEN.** `InfantryClass__Fire_At_Override` 0x0051DF70 has
  no xrefs at all, so its vtable slot cannot be bound. Settle by locating the InfantryClass
  vtable base via the RTTI Complete Object Locator preceding the table, then reading base+0x3C0
  and +0x3CC.
- **Label candidate, not applied:** 0x0041A9E0 is named `AircraftClass__What_Weapon_Should_I_Use`
  but occupies AircraftClass vtable slot +0x3C0 — the GetFireError slot — and calls
  `TechnoClass__GetFireError` at 0x0041A9F6. Looks like drift; its arity was never enumerated,
  so it was left alone.
- A prior plate comment on `BulletClass__ResolveImpactCoordAndDetonate` 0x00468D80 records
  `[this+0xD0]` as an unknown CoordStruct with "no writer found" and says not to port step 4.
  That is now stale: +0xD0 is the target-coordinate snapshot inside the +0xB8 tracker, zeroed
  from the constructor and written with the live target coord by 0x004E1130, bound to this
  receiver by `LEA ECX,[EBX+0xB8]; CALL 0x004E1130` at 0x00468A8D. Correct that comment when
  next touching it.
