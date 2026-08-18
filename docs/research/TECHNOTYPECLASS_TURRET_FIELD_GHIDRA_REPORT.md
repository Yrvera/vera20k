# TechnoTypeClass+0xCA1 — `Turret` field — Ghidra Research Report

**Address(es):**
- `0x00844110` — INI key string `"Turret"`
- `0x007133A0` — `CCINIClass::ReadBool("Turret", default)` call inside `TechnoTypeClass::ReadINI`
- `0x007133C2` — `MOV byte ptr [EBP+0xCA1], AL` (write of ReadBool result into the field)
- `0x00710AF0` — `TechnoTypeClass::Constructor`; zero-init at `*(undefined1 *)((int)param_1 + 0xCA1) = 0`
- `0x004527D0` — `BuildingClass::HasTurret` (canonical reader; reads `[Type+0xCA1]` and `[Occupant->Type+0xCA1]`)
- ~60 byte-pattern hits for displacement `0xCA1` across the binary — see Section 5

**Confidence:** HIGH (direct ReadINI string xref + literal `HasTurret` function reading the same offset; ~60 readers all consistent with a turret-presence bool)

**Active in YR:** Yes — pervasively. Read in every UnitClass, BuildingClass, and TechnoClass tick path that needs to know whether a unit has a separately-rotating turret.

---

## 1. Overview

`TechnoTypeClass+0xCA1` is a single byte holding the **`Turret`** bool from the
unit's INI section. Default `false`. It answers the question "does this unit
type have a separately-rotating turret?" and is read across draw, combat,
locomotion, facing, and AI code paths to gate turret-specific logic.

This field has been **mislabeled** in two existing reports
(`MCV_DEPLOY_GHIDRA_REPORT.md` called it `Deployer`,
`DRIVE_LOCOMOTION_CLASS.md` called it `deploy_while_moving`). Both are wrong.
The `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md` label `Turret` is correct.

---

## 2. Class layout / key offsets

### TechnoTypeClass

| Offset | Type | INI key | Default | Notes |
|---|---|---|---|---|
| `+0xCA1` | byte (bool) | `Turret=` | `0` (false) | Constructor zero-inits at `0x00710AF0`. Parsed via `CCINIClass::ReadBool` with the current field value as default. |

### Related neighbour bytes (for context, not part of this finding)

| Offset | Field | Notes |
|---|---|---|
| `+0xCA2` | byte | also zero-init'd alongside +0xCA1 — purpose unknown, possibly `IsTurretCheckable` or padding |
| `+0xD21` | byte | tested alongside `+0xCA1` in `UnitClass::Facing_Update` — controls "facing snap" timer behaviour |
| `+0xD22` | byte | written from a different `ReadBool` immediately before the Turret ReadBool — key not yet identified |

### TechnoClass (instance offsets used by readers)

| Offset | Field | Notes |
|---|---|---|
| `+0x6C4` | `Type*` | Loaded as `[ESI+0x6C4]` before reading `+0xCA1` in UnitClass/TechnoClass paths |

### BuildingClass (instance offsets used by readers)

| Offset | Field | Notes |
|---|---|---|
| `+0x520` | `Type*` | BuildingClass uses a different TypeClass slot than TechnoClass |
| `+0x5EC` | occupant ptr array | Occupant `Type+0xCA1` is also tested by `HasTurret` |
| `+0x702` | byte | occupant count (cap: byte, max 255) |

---

## 3. Core logic — how the field is consumed

### 3.1 `BuildingClass::HasTurret` (`0x004527D0`)

```
bool HasTurret(this):
    if (this->Type->Turret != 0):              // [Type + 0xCA1]
        return 1
    if (this->OccupantCount == 0):              // byte at [+0x702]
        return 0
    for i in 0 .. OccupantCount-1:
        occ = this->Occupants[i]                // [+0x5EC + i*4]
        if (occ != NULL && occ->Type->Turret != 0):
            return 1
    return 0
```

**Tiny details that matter:**
- The occupant scan **stops at the first turreted occupant** — short-circuit OR.
- `OccupantCount` is read as `char` (signed byte), but the loop bounds use it
  directly; if it ever exceeded `0x7F` the iteration would terminate early due
  to sign comparison. In practice occupants cap at 5 or so.
- Occupants live in a fixed-size array (the array at `+0x5EC` is indexed by
  pointer increment of 4 bytes — confirmed in the do-while loop). Slot 0 is
  the first occupant; null slots are valid mid-array (the loop checks for it).
- This function is the **literal proof** that `+0xCA1` is the Turret flag —
  the function's name is `HasTurret` and it does nothing but read this byte.

### 3.2 `TechnoTypeClass::ReadINI` writer (`0x007133A0` → `0x007133C2`)

```
0x713386: AL = ReadBool("<prev-key>", default=??)   ; previous boolean field
0x71338B: CL = [this + 0xCA1]                       ; load OLD Turret value (as default)
0x713391: [this + 0xD22] = AL                        ; store previous ReadBool into 0xD22
0x713397: push CL                                    ; default = old Turret value
0x713398: push 0x00844110 = "Turret"                 ; INI key name
0x71339D: push EBX (section)
0x71339E: ECX = this (INIClass*)
0x7133A0: AL = CCINIClass::ReadBool(...)             ; read "Turret"
0x7133A5: EDI = [this + 0x1580]                       ; unrelated, image-loading state
... stack setup for next ReadString ...
0x7133BA: push 0x008440FC = "TurretRotateSound"
0x7133C2: [this + 0xCA1] = AL                        ; *** store Turret into field ***
0x7133C8: ReadString("TurretRotateSound", ...)       ; reads a sound name, separate field
```

**Tiny details that matter:**
- The `MOV [this+0xCA1], AL` at `0x7133C2` stores the result of the **earlier**
  ReadBool at `0x7133A0`, not anything related to the `TurretRotateSound`
  string that's already been PUSHed on the stack a few instructions earlier.
  This is why an unwary reader (myself, last session) can mis-attribute the
  field to `TurretRotateSound`.
- The default fed into ReadBool is the **current field value** (preserved across
  ReadINI re-invocations on the same INI section), so re-parsing the same
  section with the key absent keeps the previous value rather than reverting
  to false. Standard CCINIClass::ReadBool behaviour.
- The neighbouring field at `+0xD22` is written from a **different** prior
  ReadBool whose key string we did not chase. It is NOT the same as Turret.

### 3.3 The constructor default

`TechnoTypeClass::Constructor` at `0x00710AF0` zero-inits:
```
*(undefined1 *)((int)param_1 + 0xca1) = 0;
*(undefined1 *)((int)param_1 + 0xca2) = 0;
```

So a freshly-constructed TechnoTypeClass has `Turret = false`. INI parsing
later flips it for units with `Turret=yes` in their section. The neighbour
byte at `+0xCA2` is also zero-init'd here; its identity is not investigated.

### 3.4 Where `Turret` gates draw paths

#### `UnitClass::Draw_Body_And_Turret` (vtable slot 0x1C8 @ `0x0073C5F0`)

```
if (Type->Turret != 0):
    # TURRETED-UNIT DRAW PATH
    CC_Draw_Shape(...)                         # SHP shadow for the body
    swap g_PrimarySurface to alt surface
    Matrix3x4_Copy(locomotor_matrix)           # body orientation
    Matrix_shear_col3_by_col0(...)             # slope/tilt correction
    Matrix3x4_RotateZ(body_yaw)
    Matrix_rotate_y_axis(...)
    FUN_005ae8f0(...)                          # matrix multiply
    this->vtable[0x50C](body_args)             # *** draw body voxel ***
    # Build TURRET matrix on top of body matrix
    if (Type->byte[0x29E] != 0): wobble turret yaw with RateTimer
    FUN_005ae8f0(...)                          # body * turret_offset
    FUN_005ae8f0(...)                          # again
    this->vtable[0x50C](turret_args)           # *** draw turret voxel ***
    return                                      # NEVER reads Type->TooBigToFitUnderBridge
else:
    # NO-TURRET DRAW PATH
    if (Type->TooBigToFitUnderBridge != 0
        && IsOnBridge_ForFiring()
        && bridge_piece_neighbor_count() == 0):
        # special: skip vtable+0x2F0 pre-call, Z bias = -16
        this->vtable[0x50C](z_bias = -16, pre = 0, palette = 0x100, ...)
    else:
        # normal single-pass body draw
        pre = this->vtable[0x2F0]()
        this->vtable[0x50C](z_bias = 0, pre = pre, palette = 0x100, ...)
```

**Tiny details:**
- The turret path calls vtable+0x50C **twice** (body, then turret). The
  no-turret path calls it **once**. So `Turret` controls a literal extra draw
  pass per frame.
- The turret-yaw "wobble" at runtime is gated by a separate byte at
  `Type+0x29E` (read inside the turreted-unit path). We didn't chase its INI
  key but the in-game effect is the turret's idle micro-rotation animation.
- The two `FUN_005ae8f0` calls in sequence in the turret path multiply the
  body matrix by the turret-offset matrix in two stages — first to translate
  the turret pivot, then to compose the rotation. Order matters; swapping
  produces a turret drawn at the body's origin.
- The vtable+0x50C call slot is the same for body and turret — only the
  stack args differ (different shape/frame/orientation matrix). The actual
  voxel-vs-SHP dispatch happens INSIDE vtable+0x50C, not here.

#### `UnitClass::Draw_Sprite_With_BridgeFudge` (vtable slot 0x1CC @ `0x0073B140`)

This function does **not** check `Turret` before reading TooBigToFitUnderBridge.
So even turreted units (tanks, surface ships) have their **shadow/sprite blit**
modified by TooBig when on a bridge edge — only the **main draw** at the
sibling vtable slot skips them.

### 3.5 Where `Turret` gates non-draw logic

| Function | Address | Role |
|---|---|---|
| `UnitClass::Facing_Update` | `0x00736990` | Reads at `0x00736ADE`, `0x00736BEB`, `0x00736F8A`. Decides between turreted (separate turret facing tracker) vs body-only facing. Also gates a CDTimer write to `[unit + 0x4A0]` (the turret aim cooldown). |
| `UnitClass::Fire_At_Target` | `0x00736DF0` | Reads at `0x00736F8A`. Reads `Type+0xE11` (Voxel/SHP-inverse) AND `Type+0xCA1` (Turret) — distinct combinations gate different muzzle-offset and recoil logic. |
| `UnitClass::DrawPips` | `0x0073B500` | Reads at `0x0073B7A5`. Affects pip layout, possibly garrison occupant indicator vs cargo pips. |
| `TechnoClass::AI_Update` | `0x006F9E50` | Reads at `0x006F9FAB`. Branches on Turret to handle turret AI tick (target acquisition for turret independently from body facing). |
| `ShipLocomotionClass::Process_Drive_Track` | `0x006A05F0` | Reads at `0x006A062D`. Naval-specific turret stabilization during ship rotation. |
| `BuildingClass::HasTurret` | `0x004527D0` | Canonical "is turreted?" query, also scans garrison occupants. |
| `BuildingTypeClass_ReadINI_Water` | `0x0045FE50` | Reads at `0x0046104A` as a **default** for a different field. Uses the SBB trick `NEG DL; SBB EDX, EDX; AND EDX, 0x1F; INC EDX` → produces `1` if no turret, `0x20` (32) if has turret. Likely a default for `Facings=` or `TurretFacings=`. |

There are also reads from BuildingClass methods at `0x0063XXXX` (a dense cluster
around `0x0063A000`-`0x0063AFE7` — at least 20 hits) which gate building
turret-rotation / target-acquisition / firing-anim logic. We did not trace each
individually because the field identity is already established beyond doubt.

---

## 4. INI keys

| Key | Section type | Type | Default | Effect |
|---|---|---|---|---|
| `Turret=` | `[UnitType]`, `[BuildingType]`, `[InfantryType]?`, `[AircraftType]?` | bool | `no` | Marks the type as having a separately-rotating turret. Gates the extra render pass in the unit draw method, separate turret facing/aim tracking, building turret rendering, and ship turret stabilization. |

### Cross-reference: which YR units that set TooBigToFitUnderBridge=yes also set Turret

From `ini/rulesmd.ini` (the authoritative YR data):

**`TooBig=yes` AND `Turret=no`** — these units hit the TooBig main-draw Z-bias path AND the shadow split-blit:
- Naval: `DEST` (Destroyer), `CDEST`, `CARRIER` (Aircraft Carrier), `DRED` (Dreadnought), `SUB` (Typhoon Sub), `BSUB` (Boomer), `SQD` (Giant Squid), `CRUISE`, `TUG`
- Big land: `MGTK` (Mammoth Tank — yes, MGTK is Turret=no in YR), `V3`, `HOWI`, `TNKD`, `SAPC`, `VLAD` (Apocalypse), `LCRF` (Ranger)
- Drones: `DNOA`, `DNOB`, `DRON`
- Misc: `YHVR` (Slave Miner)

The inline INI comments on `DEST` and `CDEST` say:
> `Turret=no ; can't have a turrett and a NoSpawnAlt (both go in AuxVoxel)`
The same constraint applies to `DRED`, `CARRIER`. This is a **gamemd content
quirk** — even though these ships visually do rotate guns, the engine models
them with `Turret=no` and packs the rotating gun into the `AuxVoxel` slot.

**`TooBig=yes` AND `Turret=yes`** — these units hit ONLY the shadow split-blit
(the main-draw TooBig path is skipped because the turret branch wins):
- Tanks: `MTNK`, `HTNK` (Apocalypse), `LTNK`, `UTNK`, `TTNK` (Titan),
  `YTNK`, `ROBO`, `TELE`, `MIND`, `DISK`, `XCOMET`, `FV` (IFV)
- Misc: `SREF`, `SCHP`, `SCHD`, `HARV`, `HTK`, `SMIN`

### Note on misleading INI comments

`[FV]` has the comment `Turret=yes ;GEF should be no for ifv???` — gamemd
content's own author was uncertain whether IFV should be `Turret=yes` or `no`.
This is unrelated to our findings but worth flagging: the INI is shipping with
a known-uncertain value here.

---

## 5. Integration points

- **Parser:** `TechnoTypeClass::ReadINI` @ `0x00712170` — every UnitType,
  AircraftType, BuildingType inherits this read path.
- **Constructor:** `TechnoTypeClass::Constructor` @ `0x00710AF0` — zero-init,
  default `Turret = false`.
- **Runtime readers** (verified via byte-pattern audit for displacement
  `0xCA1`, ~60 hits — sampled set classified by enclosing function):

  | Subsystem | Function(s) | Count |
  |---|---|---|
  | Draw / render | `UnitClass::Draw_Body_And_Turret`, `Draw_Sprite_With_BridgeFudge` (via TooBig sibling), `UnitClass::DrawPips`, `BuildingClass` draw hits @ `0x0063XXXX` cluster | ~25 |
  | Combat | `UnitClass::Fire_At_Target`, `TechnoClass::AI_Update` | 4 |
  | Locomotion / facing | `UnitClass::Facing_Update`, `ShipLocomotionClass::Process_Drive_Track` | 4 |
  | Building queries | `BuildingClass::HasTurret` (canonical), `BuildingTypeClass_ReadINI_Water` (default-supplier) | 4 |
  | ReadINI / constructor / destructor | `TechnoTypeClass::ReadINI`, `TechnoTypeClass::Constructor` | 4 |
  | Tile blitter (false positive — instruction immediate, not a load) | `TMP_TileBlitter` @ `0x00548479` | 1 |
  | Misc / not yet classified | other `0x6F`/`0x70`/`0x71` area hits | ~15 |

- **Tick ordering:** Most reads occur in per-unit tick phases (AI, facing,
  combat) and per-frame render passes. The field is a static type-time bool —
  it never changes after `TechnoTypeClass::ReadINI` completes during map load.

---

## 6. Current Rust implementation status

Not surveyed for this report. The Rust port is expected to already model
`Turret` in some form (probably in `src/rules/unit_type.rs` or equivalent),
since turret rotation and combat aim are core mechanics.

What this investigation **does** mean for the Rust port:

1. The previous TooBig report incorrectly described `+0xCA1` as a "SHP-vs-voxel
   dispatch byte". That framing was wrong: it's the **Turret** flag. The
   draw function `FUN_0073C5F0` dispatches on whether the unit has a turret —
   turreted units get an extra render pass with separate turret matrix
   computation; non-turreted units get a single render pass that the TooBig
   Z-bias modifies.
2. The set of YR units affected by the TooBig main-draw Z-bias is *not* limited
   to SHP units like Dolphin. It includes the **entire YR capital-ship roster**
   (DEST, CDEST, CARRIER, DRED, SUB, BSUB, SQD, CRUISE, TUG, MGTK, V3, HOWI,
   TNKD, SAPC, VLAD, etc.) — any unit type that ships with `Turret=no`. The
   "AuxVoxel exclusion" content choice in `rulesmd.ini` makes most big ships
   `Turret=no`.
3. The TooBig shadow split-blit at the sibling vtable slot does NOT dispatch
   on Turret, so turreted tanks also get their shadow rendered with the
   split-blit when on a bridge edge.

---

## 7. Open questions

1. **`Type+0xCA2`** — neighbour byte zero-init'd alongside `+0xCA1` in the
   TechnoTypeClass constructor. Identity unknown.
2. **`Type+0xD22`** — written from a different `ReadBool` immediately before
   the Turret read in ReadINI. Identity unknown.
3. **The `0x29E` byte in the turreted-unit path of
   `UnitClass::Draw_Body_And_Turret`** — gates a runtime turret-yaw wobble via
   RateTimer. Likely an INI key like `TurretCount=` or `IdleSpinTurret=`.
4. **The dense `0x0063A000`-`0x0063AFE7` reader cluster** — BuildingClass
   methods that read `Type+0xCA1`. ~20 hits in a small range, suggesting a
   loop over building turret slots. We didn't decompile each; the field
   identity doesn't depend on understanding them.
5. **Field at `+0xCA1` for InfantryTypeClass / AircraftTypeClass** — the INI
   key `Turret=` may be parsed in other ReadINI overrides too. We confirmed
   TechnoTypeClass writes it (which suffices for everything derived from
   TechnoType), but did not verify whether aircraft/infantry override the
   read or share the same offset.

---

## Sources

**Ghidra decompilation (live, this session):**
- `0x00844110` — INI key string `"Turret"`
- `0x00712170` — `TechnoTypeClass::ReadINI`
- `0x00710AF0` — `TechnoTypeClass::Constructor` (the one that zero-inits +0xCA1)
- `0x004527D0` — `BuildingClass::HasTurret`
- `0x0073C5F0` — `UnitClass::Draw_Body_And_Turret` (renamed in this session)
- `0x0073B140` — `UnitClass::Draw_Sprite_With_BridgeFudge` (renamed in this session)
- `0x00736990` — `UnitClass::Facing_Update`
- `0x00736DF0` — `UnitClass::Fire_At_Target`
- `0x006F9E50` — `TechnoClass::AI_Update`
- `0x006A05F0` — `ShipLocomotionClass::Process_Drive_Track`
- `0x0045FE50` — `BuildingTypeClass_ReadINI_Water` (uses Turret as default-supplier)
- Byte-pattern audit at `A1 0C 00 00` (60 hits across the binary)

**Ghidra annotations added this session:**
- Renamed `FUN_0073C5F0` → `UnitClass__Draw_Body_And_Turret`
- Renamed `FUN_0073B140` → `UnitClass__Draw_Sprite_With_BridgeFudge`
- Plate comment on `0x0073C5F0` (Turret dispatch + TooBig)
- Plate comment on `0x0073B140` (shadow split-blit + TooBig)
- Plate comment on `0x00712170` (TechnoTypeClass::ReadINI field map)
- Plate comment on `0x004527D0` (BuildingClass::HasTurret)
- Decompiler comments at `0x007133C2`, `0x0073C725`, `0x0073CE0D`, `0x0073B1B0`
- `save_program` executed

**Doc cross-checked / contradicted:**
- `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md:477` — labeled `+0xCA1` as Turret ✓ (correct, this report confirms)
- `MCV_DEPLOY_GHIDRA_REPORT.md:47` — labeled `+0xCA1` as Deployer ✗ (WRONG; Deployer is an InfantryType field parsed at `0x0052460D`)
- `DRIVE_LOCOMOTION_CLASS.md:125` — labeled `+0xCA1` as `deploy_while_moving` ✗ (WRONG)
- `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md:105` — labeled `+0xCA1` as `HasTurretAnim` ≈ (close, but the field is Turret itself, not a derived "has turret anim" flag)
- `UNITCLASS_GHIDRA_REPORT.md:242` — labeled `+0xCA1` as Deployer ✗ (WRONG)
- `TOO_BIG_TO_FIT_UNDER_BRIDGE_GHIDRA_REPORT.md` — open-question section flagged `+0xCA1` as uncertain "SHP-vs-voxel dispatch"; superseded by this report. See update notes in that doc.

**INI files:**
- `ini/rulesmd.ini` (authoritative for YR)
- `ini/rules.ini` (RA2 base, not directly cited here)
