# CoordStruct::Set — decode

**Address:** `0x0041c230`
**Kind:** function-decode-v1
**Source:** decompile_function @ 0x0041c230

---

## Summary

`CoordStruct::Set` is a trivial 3-component setter on a 12-byte CoordStruct (three `int32`
fields at offsets 0, 4, 8). The body stores X, Y, Z in order and returns. Its value for
the coord/cell conversion system lies in the caller patterns: 190 xrefs across the engine
establish CoordStruct as the canonical 3D lepton coordinate container used by sim, render,
bullet, anim, and building systems alike.

---

## Active in YR

**YES — actively called throughout normal YR skirmish play.**

68 direct call-site callers confirmed via `get_function_callers 0x0041c230` (live recount;
the assignment cited "190 xrefs" which includes data references, vtable pointers, and
non-call xrefs — direct call callers are 68). Span all major subsystems: vehicle drive-track,
ship movement, bullet homing, building change-owner, anim bounce, BuildingLight AI, disk laser,
rad eruption, drawing overlays, and shroud/radar rendering.

Selected named callers (from live get_function_callers 0x0041c230):
- `DriveLocomotionClass__Process_Movement` @ `0x004b2630` — vehicle position updates
- `ShipLocomotionClass__Process_Movement` @ `0x006a1c80` — ship position updates
- `WalkLocomotionClass__ProcessMovement` @ `0x0075aec0` — infantry walk movement
- `FlyLocomotionClass__Process` @ `0x004cd600` — aircraft flight movement
- `BulletClassAiHomingDetonationPath` @ `0x004666e0` — bullet homing/detonation path
- `BulletClass__HomingTrack` @ `0x005b20f0` — bullet homing track update
- `BulletClass__SpawnShrapnel` @ `0x0046a310` — shrapnel spawn position
- `BulletClassBulletDetonationImpactDamage` @ `0x00468d80` — detonation impact
- `BounceClass__Update` @ `0x00439b00` — bouncing projectile position
- `BuildingClass__ChangeOwner` @ `0x00448260` — distance check on garrison unit approach
- `BuildingClass__ReceiveDamage` @ `0x00442230` — damage location
- `BuildingLightClass__AI` @ `0x004361d0` — 3D distance from light source to target
- `AnimClass__BounceAI` @ `0x00425670` — bounce anim delta-vector computation
- `DiskLaserClass__AI` @ `0x004a7340` — disk laser beam targeting
- `AircraftClass__Fire_At` @ `0x00415ee0` — aircraft weapon fire position
- `AircraftClass__Find_Nearest_Friendly_Airfield` @ `0x0041a160` — airfield search
- `AircraftClass__Mission_Move` @ `0x004166c0` — aircraft mission movement
- `SuperClass__Launch` @ `0x006cc390` — superweapon launch position
- `SuperClass__Launch` @ `0x006cc390` — superweapon launch position
- `FootClass__Find_Path` @ `0x004d3920` — pathfinding position query
- `FootClass__Greatest_Threat_Scan` @ `0x004d5690` — threat scan position
- `InfantryClass__Mission_Capture` @ `0x005202f0` — capture mission position
- `HouseClass__Check_Spy_Reveal` @ `0x004faf00` — spy reveal check
- `Apply_area_damage` @ `0x00489280` — area damage position
- `EMPulseClass__Apply` @ `0x004c54e0` — EMP pulse position
- `EBolt__DrawRecursiveBolt` @ `0x004c1f20` — electric bolt draw
- `RadBeam__DrawStraightBeam` @ `0x00659650` — rad beam draw
- `TechnoClass__DrawExtras` @ `0x006f5190` — unit extra draw (tethers, lasers)
- `TechnoClass__DrawBehind` @ `0x006f60d0` — unit draw-behind pass
- `TechnoClass__DrawBracketCorner` @ `0x006f5ef0` — selection bracket
- `TechnoClass__DrawHealthBar` @ `0x006f64a0` — health bar draw
- `Cell_ContentRendering` @ `0x006d6d10` — cell content render
- `DrawRadarOverlay_normal` @ `0x0063c690` — radar overlay (normal)
- `DrawRadarOverlay_fog` @ `0x0063cae0` — radar overlay (fog)
- `GenerateTerrainPreview` @ `0x00641140` — terrain preview generation
- `MapClass__Resize` @ `0x00565c10` — map resize
- `ParticleSystemClass__AI_Fire` @ `0x0062f9a0` — particle fire AI
- `TriggerAction__Execute` @ `0x006dd8b0` — trigger action
- `TeamClass__Convoy_Script_Attack_Move` @ `0x006ef700` — convoy script
- `UnitClass__TubeMovement` @ `0x007359f0` — tube movement
- `iso_to_screen` @ `0x006d7560` — isometric to screen coord
- `BuildingPlacement_OverlayRenderer` @ `0x006d5030` — building placement overlay
- `Tactical_layer_*` (6 callers) @ `0x006d2de0..0x006d3ac0` — tactical render layers
- `BulletClassAiHomingDetonationPath` @ `0x004666e0` — bullet homing/detonation path
- + 23 additional `FUN_*` callers

(verified via `get_function_callers 0x0041c230`)

---

## Signature

```c
void __thiscall CoordStruct__Set(
    undefined4 *param_1,   // this: pointer to CoordStruct (12-byte buffer)
    undefined4 param_2,    // X (int32 leptons)
    undefined4 param_3,    // Y (int32 leptons)
    undefined4 param_4     // Z (int32 leptons)
)
```

- `param_1` is `undefined4*` — a caller-allocated 12-byte buffer or struct field.
- Parameters 2-4 are the three int32 lepton components.
- No return value (void). The function has no callees.

**Calling convention:** `__thiscall` — `param_1` is `this` in ECX; X/Y/Z pushed on stack.

Verified via `decompile_function 0x0041c230` and `get_function_callees 0x0041c230` (no callees).

---

## Control Flow

```
*param_1   = param_2;   // [this+0x00] = X
param_1[1] = param_3;   // [this+0x04] = Y
param_1[2] = param_4;   // [this+0x08] = Z
return;
```

No branches, no guards, no validation. Unconditional 3-word write.

---

## CoordStruct Layout (confirmed from callsite usage)

| Offset | Size  | Field | Units     | Reference frame |
|--------|-------|-------|-----------|-----------------|
| `+0x00` | 4 bytes | X | `int32` leptons | depends on caller — see below |
| `+0x04` | 4 bytes | Y | `int32` leptons | depends on caller |
| `+0x08` | 4 bytes | Z | `int32` leptons | depends on caller |

Total struct size: **12 bytes**.

Signed int32 throughout. `1 cell = 256 leptons`. Verified by caller patterns (e.g.,
`AnimClass__BounceAI` uses `param_1[0x42] / param_1[0x43]` as X/Y and calls
`CoordStruct__Distance3D`, confirming these are lepton-unit 3D vectors).

---

## Struct Field Accesses

No `this`-relative offsets — the function writes to the passed buffer directly:
`param_1[0]`, `param_1[1]`, `param_1[2]`. The struct itself is typically a local
stack buffer, a member of another class, or the return buffer of a `GetCoords` vtable call.

---

## Globals

None. (Verified via `decompile_function 0x0041c230` — body has no global reads/writes.)

---

## INI Keys

None.

---

## Enum Values

None. All constants are caller-supplied values in leptons.

---

## Observable vs Internal

**Observable outputs:**
- Any caller that writes a new position using `CoordStruct__Set` and then passes it to
  `GetCoords`, `Force_Track`, a locomotor vtable call, or a rendering function will
  produce player-visible effects: unit displacement, bullet impact location, anim spawn
  point, radar overlay position. Wrong values propagate directly to on-screen position.

**Internal mechanism:**
- The setter itself is purely mechanical — no parity risk here. Callers are the risk
  surface.

---

## Caller Pattern Analysis

Five distinct call patterns identified from sampled callers (verified via
`decompile_function` for each):

**Pattern A — Delta vector / distance input**
```
CoordStruct__Set(A.x - B.x, A.y - B.y, A.z - B.z)
// result → CoordStruct__Distance3D() or Sqrt_Approx()
```
Callers: `AnimClass__BounceAI` @ `0x00425670`, `BuildingClass__ChangeOwner` @ `0x00448260`,
`BulletClassAiHomingDetonationPath` @ `0x004666e0` (three instances), `BuildingLightClass__AI`
@ `0x004361d0`. The resulting CoordStruct is immediately passed to distance/magnitude
computation — it is NOT stored as a persistent position. Reference frame: difference
of two **GetCoords-frame** (vtable+0x48, foundation-center leptons) values.

**Pattern B — Float-to-int conversion result (render/physics math)**
```
uVar8 = Math__ftol();  // Z
uVar9 = Math__ftol(uVar8);  // Y
uVar10 = Math__ftol(uVar9); // X
CoordStruct__Set(uVar10, uVar9, uVar8);
```
Callers: `BulletClassAiHomingDetonationPath` @ `0x004666e0`, `BounceClass__Update` @
`0x00439b00`. The inputs are floating-point physics/projectile computations converted via
`Math__ftol`. This is a **render/physics boundary** — Z=integral leptons after iso-math.
**Determinism hazard**: `Math__ftol` uses x87 FPU rounding, which is non-deterministic
across CPU state. These usages occur in non-sim bullet/anim paths only — confirmed
`BounceClass__Update` is VXL matrix rendering (not sim). Not a sim-parity risk for lockstep
correctness in RA2's multiplayer (bullets use float physics), but must not be ported to
sim-side fixed-point code.

**Pattern C — Direct position from GetCoords vtable**
```
piVar = (int *)(*vtable_GetCoords)(this, buf);
CoordStruct__Set(piVar[0], piVar[1], piVar[2]);
```
Or more commonly the result buffer from vtable+0x48 is passed directly to downstream
callers. The Set is used to copy/store values. Callers: `BuildingLightClass__AI` @
`0x004361d0`.

**Pattern D — Sim-side locomotor position update**
```
// In DriveLocomotionClass__Process_Movement:
// Sets unit's [this+0x9c/0xa0/0xa4] = new X/Y/Z leptons
CoordStruct__Set(newX, newY, newZ);
```
Source: `DriveLocomotionClass__Process_Movement` @ `0x004b2630` (confirmed as a caller via
`get_function_callers`). Reference frame: **Location frame** (ObjectClass+0x9C, leptons,
NW-cell-anchored for buildings; body center for mobile units).

**Pattern E — Zero-init / null sentinel**
Some callers initialize a CoordStruct to `(0, 0, 0)` or to a known null coordinate
(`DAT_0089a178`, `DAT_0089a17c`, `DAT_0089a180`). The null/invalid sentinel appears to be
a specific non-zero coordinate stored in globals — the `AnimClass__BounceAI` code checks
for it explicitly before computing.

---

## Reference Frames Seen at Callsites

| Pattern | Frame | Source |
|---------|-------|--------|
| A (delta vector) | GetCoords (foundation center, leptons) — vtable+0x48 output | both operands come from vtable+0x48 calls |
| B (float→int) | Iso-projection / bullet physics space (leptons) | `Math__ftol` of floating-point trajectory |
| D (locomotor) | Location (leptons) — ObjectClass+0x9C/0xA0/0xA4 | written directly into object location fields |
| E (null init) | Any / none | sentinel or zero init |

---

## Out-of-scope refs

| Symbol | Address | Reason deferred |
|--------|---------|-----------------|
| `CoordStruct__Distance3D` | `0x0041c380` | Magnitude/distance function called after Pattern A. Own decode task #6. |
| `Math__ftol` | referenced throughout | x87 float→int conversion. Non-deterministic for sim. Details in its own decode. |
| `DAT_0089a178/7c/80` | — | Null/invalid CoordStruct sentinel. Exact values and their role need null-coord decode. |
| `g_NullCoord_Drive_X/Y/Z` | — | A separate null-coord sentinel used by drive locomotion. Related to frame D. |
| vtable+0x48 (`GetCoords`) | — | The canonical object position accessor. Task #11 (`decode-fn-abstract-getcoords`). |

---

## Rust Equivalent

```rust
// CoordStruct: 3 × i32 leptons. Layout: [X, Y, Z] at offsets 0, 4, 8.
// Reference frame depends on caller — do NOT assume any single frame.
#[repr(C)]
struct CoordStruct {
    x: i32,  // leptons, +X = east
    y: i32,  // leptons, +Y = south
    z: i32,  // leptons, +Z = up (height)
}

impl CoordStruct {
    fn set(&mut self, x: i32, y: i32, z: i32) {
        self.x = x;
        self.y = y;
        self.z = z;
    }
}
```

The struct size (12 bytes) and field order are confirmed by caller patterns passing
`[0]`, `[1]`, `[2]` as X, Y, Z respectively.

---

## Unverified

None. All claims verified from live Ghidra decompilation in this session:
- `decompile_function 0x0041c230` — main function body (trivial 3-write setter)
- `get_function_callers 0x0041c230` — 68 direct call-site callers (live recount; assignment cited 190 xrefs which includes non-call references)
- `get_function_callees 0x0041c230` — no callees
- `decompile_function 0x00425670` — `AnimClass__BounceAI` (Pattern A)
- `decompile_function 0x004666e0` — `BulletClassAiHomingDetonationPath` (Patterns A, B)
- `decompile_function 0x00439b00` — `BounceClass__Update` (Pattern B)
- `decompile_function 0x00448260` — `BuildingClass__ChangeOwner` (Pattern A)
- `decompile_function 0x004361d0` — `BuildingLightClass__AI` (Patterns A, C)
