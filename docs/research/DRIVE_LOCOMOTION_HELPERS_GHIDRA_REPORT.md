# DriveLocomotionClass Helper Functions — Ghidra Research Report

**Primary Address Range:** `0x004AF3E0` – `0x004B4DE0`
**Object Size:** 0x6C bytes (108 bytes)
**CLSID:** `{4A582741-9839-11d1-B709-00A024DDAFD1}`
**Confidence:** HIGH (all functions decompiled from binary)
**Active in YR:** Yes — used by 58 vehicle types (Grizzly, Prism Tank, etc.)

## 1. Overview

DriveLocomotionClass implements the ILocomotion COM interface for wheeled/tracked ground
vehicles. It manages curved movement along pre-computed track tables, slope visual
interpolation, bridge height transitions, occupation bit marking, and VXL matrix
generation. This report covers every helper function in the class, decompiled from
gamemd.exe and cross-referenced against six existing research documents.

**TS Legacy Note:** `Acquire_Hunter_Seeker_Target` is a stub (empty function body).
Hunter Seeker is a Tiberian Sun unit — this method is dead code in YR.

---

## 2. Object Layout (Verified from Constructor + All Methods)

Constructor at `0x004AF540`. Base class LocomotionClass constructed at `0x0055A6C0`.

### LocomotionClass Base (inherited, +0x00 to +0x17)

| Offset | Size | Init | Field | Evidence |
|--------|------|------|-------|----------|
| +0x00 | 4 | vtable | `IUnknown_vftable` | Constructor sets `0x007E7F7C` |
| +0x04 | 4 | vtable | `ILocomotion_vftable` | Constructor sets `0x007E7EB0` |
| +0x08 | 4 | 0 | `owner_ref` (FootClass*) | Set_Destination vtable calls via `*(this+8)` |
| +0x0C | 4 | 0 | `owner_obj` (FootClass*) | Apply_Track_Delta, Stop_And_Scatter use `*(this+0xC)` |
| +0x10 | 1 | 1 | `is_active` flag | Is_Moving_Now vtable call via `*(this+0x10)` |
| +0x11 | 1 | 1 | `is_powered` flag | LocomotionClass base |
| +0x14 | 4 | 0 | `(reserved)` | LocomotionClass base |

### DriveLocomotionClass Extensions (+0x18 to +0x6B)

> **NOTE:** Offsets in this table are from the **ILocomotion interface** (object_base + 4),
> since the helper functions are called through the ILocomotion vtable. To convert to
> absolute object_base offsets (as used in DRIVE_LOCOMOTION_CLASS.md), add 4.
> E.g., `+0x30` here = absolute `+0x34` (destination.X).

| Offset | Size | Init | Field | Evidence |
|--------|------|------|-------|----------|
| +0x18 | 4 | vtable | `IPiggyback_vftable` | Constructor sets `0x007E7E8C` |
| +0x1C | 4 | 0 | `cached_slope_index` | Process reads CellClass+0x11C here |
| +0x20 | 4 | frame | `slope_start_frame` | Force_New_Slope sets to `g_CurrentFrameCounter` |
| +0x24 | 4 | undef | `slope_timer_param` | Draw_Matrix/Shadow_Matrix read |
| +0x28 | 4 | undef | `slope_timer_remaining` | Draw_Matrix: frames remaining in transition |
| +0x2C | 4 | 0 | `slope_timer_duration` | Draw_Matrix: total frames (always 3 when active) |
| +0x30 | 4 | 0 | `dest_x` | Set_Destination, Is_Moving |
| +0x34 | 4 | 0 | `dest_y` | Set_Destination, Is_Moving |
| +0x38 | 4 | 0 | `dest_z` | Set_Destination (bridge Z added here) |
| +0x3C | 4 | 0 | `head_x` | Head_To_Coord, Stop_Moving, Mark_All_Occupation_Bits |
| +0x40 | 4 | 0 | `head_y` | Head_To_Coord, Stop_Moving, Force_Track |
| +0x44 | 4 | 0 | `head_z` | Head_To_Coord, Stop_Moving, Force_Track |
| +0x48 | 4 | 0 | `(unknown_coord)` | Constructor sets to 0 |
| +0x4C | 8 | 0.0 | `current_speed` (double) | Stop_Moving clamps to 0.3; Force_Track sets to 1.0 |
| +0x54 | 4 | 0 | `track_index` | Force_Track, Can_Use_Track (index into TurnTrack[72]) |
| +0x58 | 4 | -1 | `raw_track_lookup` | Process_Drive_Track, Can_Use_Track, Transform_Track_Coords |
| +0x5C | 4 | -1 | `track_point_index` | Apply_Track_Delta, Is_To_Have_Shadow_Override |
| +0x5F | 1 | 0 | `is_on_track` | Force_Track sets to 1 |
| +0x60 | 1 | 0 | `use_short_track` | Can_Use_Track checks this |
| +0x61 | 1 | 0 | `(unknown flag)` | Constructor |
| +0x62 | 1 | 0 | `was_waiting_flag` | Process (tracks drive delay timer state) |
| +0x63 | 1 | 0 | `(unknown flag)` | Constructor |
| +0x64 | 1 | 0 | `(unknown flag)` | Constructor |
| +0x65 | 1 | 1 | `piggyback_ready` | Is_Ok_To_End checks this; init to 1 |
| +0x68 | 4 | 0 | `piggybacked_locomotor` (ILocomotion*) | Begin/End_Piggyback, Is_Piggybacking |

**IPiggyback `this` adjustment:** IPiggyback methods receive `this = object_base + 0x18`.
So `*(this + 0x50)` in IPiggyback methods = `object_base + 0x68` (piggybacked_locomotor).
Confirmed by Is_Ok_To_End accessing `this - 0x14` to reach the ILocomotion vtable at +0x04.

---

## 3. Helper Function Catalog

### 3.1 Height & Bridge Initialization (Static Functions)

These are static (classless) initialization functions called once during map load. They
compute global constants used by all DriveLocomotionClass instances.

#### StoreTileHeight — `0x004AF3E0`
```
g_TileAngleBase (0x8A0758) = DEG_TO_RAD (0x7E6238) × 90.0 (0x7E1730)
                            = π/180 × 90.0
                            = π/2  (1.5707963...)
```
**Purpose:** Stores the base angle (90° in radians = π/2) used as the reference for
slope height calculations. This is half a right angle — the maximum possible slope.

#### InitHeightStep_A — `0x004AF400`
```
g_DriveHeightStep = ftol(sin(g_TileAngleBase - g_TileAngleCurrent))
                  = ftol(sin(π/2 - current_angle))
                  = ftol(cos(current_angle))
```
**Purpose:** Computes the integer height step from the tile geometry angle. Uses the
sine-complement identity `sin(π/2 - θ) = cos(θ)`. The result is the lepton height
per elevation level for the current map's tile dimensions.

**Globals:**
- `g_TileAngleCurrent` at `0x8A0780` — runtime-computed angle based on actual tile height/width
- `g_DriveHeightStep` — the computed integer height step; used everywhere in the drive system
- `Sin_Lookup_Table4096` — 4096-entry sine lookup table at `0x8610B4`

#### ComputeFromHeightStep — `0x004AF440`
```
g_SlopeAngle (0x8A0788) = sin_lookup(g_DriveHeightStep × (1/256))
```
**Purpose:** Computes a fractional slope angle from HeightStep scaled by 1/256 (the
lepton-to-cell ratio). Stored as a double for use in slope rendering.

**Constant:** `_DAT_007e1740` = `1/256 = 0.00390625` (double)

#### ComputeBridgeRenderOffset — `0x004AF470`
```
g_BridgeRenderAngle (0x8A07A0) = sin_lookup((g_DriveHeightStep × 2) / g_TileDivisor)
```
**Purpose:** Computes the render-space vertical offset angle for bridge deck rendering.
Uses `HeightStep × 2` (bridge is 2× height) divided by a tile-geometry divisor.

**Global:** `g_TileDivisor` at `0x8A0778` — runtime-computed from map tile geometry.

#### ComputeBridgeZOffset — `0x004AF4A0`
```
g_BridgeZOffset_Drive = ftol(g_DriveHeightStep × 4)
```
**Purpose:** Bridge Z offset in leptons = 4 height levels. This is THE critical constant
for bridge layer detection. Integer truncation via `Math__ftol`.

#### InitHeightStep2 — `0x004AF500`
```
g_HeightCounter (0x8A07C0) = 0
g_CellCenterX   (0x8A07B8) = 0x80  (128 = cell center X)
g_CellCenterY   (0x8A07BC) = 0x80  (128 = cell center Y)
```
**Purpose:** Resets cell-center offset tracking. 128 = half of 256 leptons per cell edge.

---

### 3.2 ILocomotion Interface Methods

#### Set_Destination — `0x004AFD40`

**Signature:** `Set_Destination(this, coord_x, coord_y, coord_z)`

**Guard checks (4 vtable calls on owner at +0x08):**
1. `owner->vtable[0xDF]` — Is_Deploying? Returns early if true.
2. `owner->vtable[0xE0]` — Is_Unloading? Returns early if true.
3. `owner->vtable[0x75]` — Is_Sinking? Returns early if true.
4. `owner->vtable[0x76]` — Is_Falling? Returns early if true.

**Core logic:**
```
if dest != NullCoord:
    this.dest = coord
    cell = CellClass::Get_Cell_At(coord)
    if cell.flags (at cell+0x140) & 0x100:    // bridge cell flag
        this.dest_z += g_BridgeZOffset_Drive   // assume bridge level
```

**Key insight:** Destination Z is unconditionally increased for bridge cells. The unit
always assumes it WILL be on the bridge at the destination. Runtime height-threshold
checks (`abs(diff) < 2`) handle the case where the unit is actually underneath.

**Active in YR:** Yes.

#### Stop_Moving — `0x004AFE00`

**Logic:**
```
if head_coord != NullCoord:
    owner_type = owner->vtable[0x21]()  // GetTypeClass
    if owner_type+0xC94 != 0            // has convoy chain flag
       AND owner+0x6D0 == 0             // not convoy leader
       AND owner+0x6C8 != 0:            // has followers
        // Propagate stop to convoy followers
        for each follower in chain (via +0x6C8 linked list):
            follower->locomotor->vtable[0x12]()  // Stop_Moving on each

// Clamp speed to min(0.3, current_speed)
this.current_speed = min(0.3, this.current_speed)

// Clear destination
this.dest = NullCoord
```

**Constants:**
- `0.3` (double at `0x7E6240`) — minimum braking speed; prevents instant stops

**Convoy logic:** When a convoy leader stops, it propagates `Stop_Moving` to all
followers via a linked list at `owner+0x6C8`. Each iteration calls
`follower->locomotor->Stop_Moving()`.

**Active in YR:** Yes (convoy logic only active for units with Accelerates=true in INI).

#### Is_Moving (ILocomotion) — `0x004AFB80`

**Logic:**
```
if dest != NullCoord:
    return true
if head_coord == NullCoord:
    return false
if head_coord.x == owner.position.x (at owner+0x9C)
   AND head_coord.y == owner.position.y (at owner+0xA0):
    return false    // arrived at head target
return true
```

**Active in YR:** Yes. Three-check pattern: has destination → has head target → at target.

#### Is_Moving_Now — `0x004AFC20`

**Logic:**
```
if CDTimerClass::Remaining() != 0:    // slope transition in progress
    return true
if is_active (vtable[4] on ILocomotion at +0x04)
   AND dest != NullCoord:
    speed_value = owner->vtable[0x14E]()  // GetCurrentSpeed?
    return speed_value > 0
return false
```

**Purpose:** More immediate than Is_Moving — checks if the unit is actively moving
THIS tick (has budget, has speed, has destination).

**Active in YR:** Yes.

#### Destination — `0x004AFC90`

**Logic:**
```
out->x = this.dest_x    (+0x30)
out->y = this.dest_y    (+0x34)
out->z = this.dest_z    (+0x38)
return out
```

**Trivial getter.** Returns the destination coordinate. No transformation.

#### Head_To_Coord — `0x004AFCC0`

**Logic:**
```
if head_coord == NullCoord:
    // No track target — return owner's current position
    out->x = owner->position.x  (owner+0x9C)
    out->y = owner->position.y  (owner+0xA0)
    out->z = owner->position.z  (owner+0xA4)
else:
    out->x = this.head_x  (+0x3C)
    out->y = this.head_y  (+0x40)
    out->z = this.head_z  (+0x44)
return out
```

**Purpose:** Returns the coordinate the unit is currently heading toward. If no track
is active, returns the unit's own position (stationary).

**Active in YR:** Yes.

#### Do_Turn — `0x004B0EF0`

**Logic:**
```
RateTimer::Set(desired_facing)
```

**Trivial wrapper.** Sets the facing rate timer to the desired facing value.
The actual rotation happens via `RateTimer::Current()` over subsequent ticks.

**Active in YR:** Yes.

#### Get_Status — `0x004B4C60`

```
return 0;    // always returns 0 (STATUS_IDLE)
```

**Stub.** Drive locomotion always reports idle status. No state machine exposed.

#### Is_Surfacing — `0x004B4C80`

```
return false;    // always returns false
```

**Stub.** Drive vehicles never surface (that's for submarines). Dead for ground units.

#### Acquire_Hunter_Seeker_Target — `0x004B4C70`

```
return;    // empty function body
```

**TS Legacy — Dead Code.** Hunter Seeker is a Tiberian Sun unit. This is a no-op in YR.
**Active in YR:** No.

---

### 3.3 Slope & Facing Visual Functions

#### Force_New_Slope — `0x004AFB40`

**Signature:** `Force_New_Slope(this, new_slope_index)`

**Logic:**
```
this.cached_slope_index  (+0x1C) = new_slope_index
this.previous_slope      (+0x18) = new_slope_index  // same — no transition
this.slope_start_frame   (+0x20) = g_CurrentFrameCounter
this.slope_timer_param   (+0x24) = <uninitialized local>
this.slope_timer_remaining (+0x28) = 0
this.slope_timer_duration  (+0x2C) = 0
```

**Purpose:** Forces the slope to a specific index WITHOUT triggering a transition
animation. Sets both old and new slope to the same value and zeroes the timer.
Used when placing units or teleporting — the unit should snap to the new slope
immediately rather than smoothly interpolating.

**Note:** +0x24 is written from an uninitialized local variable. This is benign because
duration (+0x2C) is set to 0, so the timer is never consulted.

**Active in YR:** Yes.

#### Update_Facing_From_Type — `0x004B04D0`

**Signature:** `Update_Facing_From_Type(this)` where this is ILocomotion pointer.

**Logic:**
```
iLocomotion_vtable = *this          // ILocomotion vtable at +0x04
cell = owner->vtable[0x6F]()       // GetOccupiedCell (vtable offset 0x1BC)
slope_index = cell+0x11C           // CellClass SlopeIndex byte
this->vtable[0x1F](slope_index)    // ILocomotion::Force_New_Slope (offset 0x7C)
```

**Purpose:** Reads the current cell's slope index and forces the locomotor to match.
Called when a unit first enters a cell or needs to sync its visual slope state.

**Active in YR:** Yes.

---

### 3.4 Matrix Generation (VXL Rendering)

#### Draw_Matrix — `0x004AFF60`

**Signature:** `Draw_Matrix(this, out_matrix_3x4, inout_slope_cache)`

**Two rendering paths based on conditions:**

**Simple Path** (no dynamic tilt):
- Condition: `slope_fraction == 1.0 AND abs(body_roll) < TILT_THRESHOLD AND abs(body_pitch) < TILT_THRESHOLD`
- `TILT_THRESHOLD` at `0x7E44E8` = float `89478.96` — effectively infinity for
  ground vehicles, meaning the simple path is ALWAYS taken unless slope is transitioning
- Steps:
  1. `BuildFacingRotationMatrix()` — sub-tick facing interpolation
  2. If `slope_timer_duration == 0`: use `VXL_GetFacingMatrix(cached_slope_index)` (static lookup)
  3. If transitioning: use `VXL_InterpolatedFacing(cached_slope_index, fraction)` (SLERP)
  4. Compose: `result = facing_matrix × slope_matrix`

**Complex Path** (dynamic tilt — effectively unreachable for Drive):
- Builds full rotation matrices from body_roll (`owner+0x328`) and body_pitch (`owner+0x32C`)
- Composes: `result = facing × slope × pitch_roll_tilt`
- Sets `*slope_cache = -1` to disable slope cache

**Slope transition fraction:**
```
if slope_timer_duration != 0:
    if slope_start_frame != -1:
        elapsed = g_CurrentFrameCounter - slope_start_frame
        remaining = max(slope_timer_remaining - elapsed, 0)
    fraction = (duration - remaining) / duration    // 0.0 → 1.0
else:
    fraction = 1.0    // fully transitioned
```

**Sub-tick facing interpolation** (in `BuildFacingRotationMatrix` / `Build_Shadow_Matrix`):
```
sub_facing = ((rate_timer >> 10) + 1 >> 1) & 0x1F    // 0..31
angle = (sub_facing - 8) × (-π/16)                   // ~-0.196 rad per step
Matrix3x4_RotateZ(angle)
```
**Constant:** `-π/16` at `0x7E4408` = double `-0.19634954` (= -11.25° per step)

**Slope cache computation** (in `Build_Shadow_Matrix`):
```
cache = (original_cache × 64 + slope_index) × 32
cache |= sub_facing    // encode sub-tick facing into low bits
```

**Active in YR:** Yes. The complex path is effectively dead (TILT_THRESHOLD is huge).
Slope interpolation IS active with 3-frame transitions.

#### Shadow_Matrix — `0x004B0410`

**Signature:** `Shadow_Matrix(this, out_matrix_3x4, inout_slope_cache)`

**Logic is simpler than Draw_Matrix:**
```
fraction = compute_slope_fraction()  // same formula as Draw_Matrix

if fraction != 1.0 OR abs(body_roll) >= TILT_THRESHOLD OR abs(body_pitch) >= TILT_THRESHOLD:
    if slope_cache != null:
        *slope_cache = -1    // disable cache, force full shadow recompute

matrix = LocomotionClass::Build_Shadow_Matrix(this)
out_matrix = matrix
```

**Key difference from Draw_Matrix:** Shadow_Matrix delegates to
`LocomotionClass::Build_Shadow_Matrix` (at `0x0055B120`?) which handles the isometric
shadow projection. It only overrides the cache invalidation behavior.

**Active in YR:** Yes.

#### In_Which_Layer — `0x004B4820`

```
return 2;    // Layer::Ground (always)
```

**Stub.** Drive vehicles are always in the ground layer. No bridge/air distinction
at the locomotor level — layer is determined elsewhere.

**Active in YR:** Yes.

#### Is_To_Have_Shadow_Override — `0x004B4920`

**Signature:** `Is_To_Have_Shadow_Override(this, ground_z)`

**Purpose:** Determines if the vehicle's shadow should be drawn at a different position
than normal (i.e., the vehicle is elevated above the ground due to being on a bridge
ramp or having a track delta). Returns true + adjusts the shadow height.

**Logic:**
```
head_coord = ILocomotion::Head_To_Coord()    // vtable call at +0x18
if head_coord == NullCoord: return false

// Check if currently on an active track with cell-crossing geometry
if !use_short_track AND track_index != -1:
    raw_track = TurnTrack[track_index].normal_track
    if raw_track != 0:
        entry_index = RawTrack[raw_track].entry_index
        if entry_index >= 0:
            // Read track endpoint position
            endpoint = track_points[entry_index]
            if track_point_index < entry_index:
                // Transform endpoint to world coords
                transformed = Transform_Track_Coords(endpoint)
                // Check if endpoint is in same cell as current position
                if same_cell(transformed, current_pos):
                    height_diff = abs(owner.position.y - ground_z)
                    if height_diff <= g_DriveHeightStep:
                        return true    // shadow at adjusted height

// Fallback: check head_coord directly
if same_cell(head_coord, current_pos):
    height_diff = abs(head_coord.z - ground_z)
    if height_diff <= g_DriveHeightStep:
        return true

return false
```

**Active in YR:** Yes. Handles shadow positioning for vehicles on bridge ramps.

---

### 3.5 Track & Movement Helpers

#### Force_Track — `0x004B0C40`

**Signature:** `Force_Track(this, track_index, dest_x, dest_y, dest_z)`

**Purpose:** Force-assigns a specific drive track to the unit. Used for scripted
movement and special cases (map edge retreat, forced turns).

**Logic:**
```
this.track_index (+0x54) = track_index
this.raw_track_lookup (+0x58) = 0

if dest != NullCoord:
    // Clear existing head_coord
    if head_coord != NullCoord:
        this.head_coord = NullCoord
        this.is_on_track (+0x5F) = 0

    // Validate destination cell
    this.is_on_track = 1
    this.head_coord = dest
    cell = CellClass::Get_Cell_At(dest)
    crate_ok = CrateClass::PickupDispatch(owner)  // (corrected 2026-05-29: was CellClass::Can_Enter_Cell_General(cell, owner); binary shows CrateClass__PickupDispatch called with owner pointer, not Can_Enter_Cell_General — OPERATOR_OR_ORDER_DRIFT; verified via decompile_function 0x004B0C40)

    if crate_ok == false OR owner.is_falling (+0x81):
        // Apply track delta and set up movement
        Apply_Track_Delta(dest, MARK_OCCUPY=1)
        this.dest = dest
        this.current_speed = 0.0 (lower) | 0x3FF00000 (upper) = 1.0 (double)
        // Speed set to 1.0 = full speed
    else:
        if owner.is_alive (+0x90):
            // Can't enter — abort track
            this.head_coord = NullCoord
            this.is_on_track = 0
```

**Active in YR:** Yes.

#### Can_Use_Track — `0x004B4B00`

**Signature:** `Can_Use_Track(this)`

**Purpose:** Checks if the current track configuration can be used based on the unit's
path queue direction. Returns true if the track is valid for the queued path step.

**Logic:**
```
path_dir = owner.path_queue[0] (at owner+0x5E0)

if path_dir < -1 OR path_dir > 8:
    return false    // invalid path direction

track_entry_index = this.track_index × 12
if this.use_short_track:
    raw_track = TurnTrack[track_index].short_track    // +1 offset
else:
    raw_track = TurnTrack[track_index].normal_track   // +0 offset

if path_dir != 8 AND path_dir != -1:
    track_dir = TurnTrack[track_index].target_facing
    facing_dir = (track_dir >> 12 + 1 >> 1) & 7    // convert facing to 0-7

    if facing_dir != path_dir
       AND RawTrack[raw_track].entry_index == this.raw_track_lookup
       AND this.raw_track_lookup != 0:
        // Need a different track — compose path_dir × 8 + facing_dir
        composite = path_dir + facing_dir × 8
        raw_track = TurnTrack[composite].normal_track
        if raw_track != 0 AND RawTrack[raw_track].points_count != 0:
            return true

return false
```

**Active in YR:** Yes. Critical for track chaining across cells.

#### Apply_Track_Delta — `0x004B0AD0`

**Signature:** `Apply_Track_Delta(this, coords, mode)`

**Purpose:** Applies the track endpoint position delta to the unit. Handles both
marking (placing) and unmarking (removing) the unit from cells.

**Modes:**
- `mode 0`: Unmark old, apply delta, mark new (normal cell transition)
- `mode 1 or 3`: Mark only (initial placement or forced track)

**Logic:**
```
if coords == NullCoord: return

// If on an active track with valid geometry:
if !use_short_track AND raw_track_lookup != -1:
    raw_track = TurnTrack[raw_track_lookup].normal_track
    if raw_track != 0:
        entry_index = RawTrack[raw_track].entry_index
        if entry_index >= 0 AND track_point_index < entry_index:
            // Get track endpoint and transform to world coords
            endpoint = track_points[entry_index]
            transformed = Transform_Track_Coords(endpoint)
            // Get current facing from owner
            facing = owner->vtable[0x74]()    // GetBodyFacing

            if mode == 0:
                owner->vtable[0x3D](transformed)  // Unmark old
                owner->vtable[0x3D](coords)       // Unmark new
            elif mode == 1 or 3:
                owner->vtable[0x3C](transformed)  // Mark new

// Apply base coords
if mode == 0:
    owner->vtable[0x3D](coords)    // Unmark
elif mode == 1 or 3:
    owner->vtable[0x3C](coords)    // Mark
```

**vtable methods:**
- `owner->vtable[0x3C]` (offset 0xF0) = `ObjectClass::Mark(MARK_PUT)` — place on map
- `owner->vtable[0x3D]` (offset 0xF4) = `ObjectClass::Mark(MARK_REMOVE)` — remove from map
- `owner->vtable[0x74]` (offset 0x1D0) = `ObjectClass::GetBodyFacing`

**Active in YR:** Yes.

#### Transform_Track_Coords — `0x004B4780`

**Signature:** `Transform_Track_Coords(this, out, in_point, inout_facing)`

**Purpose:** Transforms a raw track point (x, y, facing) from track-local space to
world-cell space, applying mirror/flip flags and adding the current cell origin.

**Transform flags** (from `g_DriveTrackFlags_Table` at `0x7E7B30`, indexed by
`raw_track_lookup × 12`):

| Bit | Effect on X/Y | Effect on Facing |
|-----|---------------|-----------------|
| 0 (0x01) | Swap X↔Y | facing = (-facing - 0x40) & 0xFF |
| 1 (0x02) | Negate X | facing = (-facing) & 0xFF |
| 2 (0x04) | Negate Y | facing = (-facing - 0x80) & 0xFF |

**Final step:** Add cell origin offset from `this+0x40` (head_y) and `this+0x44`
(head_z) to the transformed coordinates:
```
out.x = this.head_y + transformed_x
out.y = this.head_z + transformed_y
```

**Note:** The flags are NOT bit 3 = cell-crossing. That's a separate field in the
TurnTrack entry, not part of the transform flags.

**Active in YR:** Yes.

#### Mark_All_Occupation_Bits — `0x004B48D0`

**Signature:** `Mark_All_Occupation_Bits(this, mark_mode)`

**Logic:**
```
if head_coord != NullCoord:
    Apply_Track_Delta(head_coord, mark_mode)
```

**Trivial wrapper.** Delegates to Apply_Track_Delta with the current head coordinate.
Called when the unit needs to refresh its cell occupation state (e.g., after loading,
teleporting, or piggybacking).

**Active in YR:** Yes.

#### Stop_And_Scatter — `0x004B4890`

**Signature:** `Stop_And_Scatter(this)` — fastcall

**Logic:**
```
if owner->tether_target (at owner+0x598, index 0x166) == 0:
    // No tether — just scatter
    owner->vtable[0x120](0, 1)    // Scatter(direction=0, forced=true)
else:
    // Tethered (e.g., to refinery) — stop first, then force scatter
    FootClass::Stop_Moving()
    owner->vtable[0x121](0, 1)    // Scatter_Force(direction=0, forced=true)
```

**vtable methods:**
- `owner->vtable[0x120]` (offset 0x480) = `Scatter` — gentle scatter
- `owner->vtable[0x121]` (offset 0x484) = `Scatter_Force` — forced scatter (ignores tether)

**Purpose:** Handles the case where a unit needs to get out of the way. If tethered
(docked at refinery/repair depot), it must stop and force-scatter rather than just
gently scattering.

**Active in YR:** Yes.

---

### 3.6 Piggyback System (COM Interface)

The piggyback system allows one locomotor to temporarily override another. Used by
TeleportLocomotionClass to wrap DriveLocomotionClass — the teleport takes control,
then hands back to drive when done.

**All IPiggyback methods receive `this = object_base + 0x18`** (IPiggyback vtable offset).

#### Begin_Piggyback — `0x004AF8E0`

**Logic:**
```
if param_locomotor == null: return E_POINTER (0x80004003)
if piggybacked_locomotor (+0x68) != null: return E_FAIL (0x80004005)
piggybacked_locomotor = param_locomotor
param_locomotor->AddRef()
return S_OK (0)
```

**Active in YR:** Yes (used by Chrono Legionnaire teleport).

#### End_Piggyback — `0x004AF930`

**Logic:**
```
if out_ptr == null: return E_POINTER
if piggybacked_locomotor != null:
    *out_ptr = piggybacked_locomotor
    piggybacked_locomotor = null
    return S_OK
return 1    // no piggybacked locomotor to return
```

**Note:** Does NOT call Release() on the piggybacked locomotor — ownership transfers
to the caller.

#### Is_Ok_To_End — `0x004AF970`

**Logic (this = object_base + 0x18):**
```
is_moving = ILocomotion::Is_Moving()    // via vtable at this-0x14
if !is_moving
   AND piggybacked_locomotor (this+0x50 = obj+0x68) != null
   AND piggyback_ready (this+0x4D = obj+0x65) != 0
   AND owner.deploy_flag (owner+0x6AD) == 0:
    return true
return false
```

**Purpose:** Returns true when the drive locomotor has finished its work and the
piggybacked locomotor can take control back. Requires: stopped, has piggyback,
ready flag set, and unit not deploying.

#### Is_Piggybacking — `0x004B4CD0`

```
return piggybacked_locomotor (this+0x50 = obj+0x68) != 0
```

#### Piggybacker_CLSID — `0x004AF610`

**Purpose:** Returns the CLSID of the piggybacked locomotor (or this locomotor's own
CLSID if not piggybacking). Traverses the piggyback chain by QueryInterface for
IPiggyback on the piggybacked locomotor.

**Active in YR:** Yes.

---

### 3.7 Constructor — `0x004AF540`

**Flow:**
1. Calls `LocomotionClass::Constructor()` — sets up base vtables and fields
2. Zeroes all DriveLocomotionClass-specific fields
3. Sets `slope_start_frame = g_CurrentFrameCounter`
4. Sets `raw_track_lookup = -1`, `track_point_index = -1`
5. Sets `piggyback_ready = 1`
6. Installs three vtable pointers:
   - `+0x00` → `IUnknown_vtable` (0x7E7F7C)
   - `+0x04` → `ILocomotion_vtable` (0x7E7EB0)
   - `+0x18` → `IPiggyback_vtable` (0x7E7E8C)

---

## 4. Global Constants (Decoded from Binary)

| Address | Type | Value | Name / Purpose |
|---------|------|-------|----------------|
| `0x7E6238` | double | `0.01745329...` (π/180) | DEG_TO_RAD |
| `0x7E1730` | double | `90.0` | Right angle degrees |
| `0x7E1740` | double | `0.00390625` (1/256) | Lepton-to-cell ratio |
| `0x7E6240` | double | `0.3` | Min braking speed (Stop_Moving clamp) |
| `0x7E44E8` | float | `89478.96` | Body tilt threshold (effectively ∞ for Drive) |
| `0x7E4408` | double | `-0.19634954` (-π/16) | Sub-tick facing angle step (-11.25°/step) |
| `0x7E48F0` | double | `1.5` | Deceleration ratio (decel = accel × 1.5) |
| `0x7E3548` | double | `0.2` | Braking speed constant |
| `0x8A0758` | double | π/2 (runtime) | Tile angle base (StoreTileHeight result) |
| `0x8A0778` | double | (runtime) | Tile geometry divisor |
| `0x8A0780` | double | (runtime) | Current tile angle |
| `0x8A0788` | double | (runtime) | Computed slope angle |
| `0x8A07A0` | double | (runtime) | Bridge render angle |
| `0x8A07B8` | int | 128 (runtime) | Cell center X offset |
| `0x8A07BC` | int | 128 (runtime) | Cell center Y offset |
| `0x8A07C0` | int | 0 (runtime) | Height counter (reset) |
| `0x8A0790` | 12 bytes | (0,0,0) | NullCoord sentinel |

**NullCoord** at `0x8A0790` is `(0, 0, 0)` — checked as BSS data. All coordinate
comparisons check against this triplet.

---

## 5. Integration Points

### Who calls these functions?

| Caller | Functions Called | When |
|--------|----------------|------|
| `FootClass::AI` (0x4DA530) | `Process` → chains to `Process_Drive_Track`, `Process_Movement` | Every game tick |
| `FootClass::Draw_It` | `Draw_Matrix`, `Shadow_Matrix` | Every render frame |
| `FootClass::Stop_Moving` (0x4DF0D0) | `Stop_Moving` | Mission change, path failure |
| `FootClass::Assign_Destination` | `Set_Destination` | Player orders, AI commands |
| `LocomotionClass::Link_To_Object` | Sets +0x08, +0x0C | Unit creation |
| `TeleportLocomotionClass` | `Begin_Piggyback`, `End_Piggyback`, `Is_Ok_To_End` | Chrono teleport |
| Map initialization | `StoreTileHeight`, `InitHeightStep_A/2`, `ComputeBridgeZOffset` | Map load |

### What does Process call?

```
Process (0x4B0500)
  ├── Phase 1: Slope detection → reads CellClass+0x11C
  ├── Phase 2: Active track → Process_Drive_Track (0x4B0F20)
  │   ├── Apply_Track_Delta (0x4B0AD0)
  │   │   └── Transform_Track_Coords (0x4B4780)
  │   ├── CellClass methods (Get_Center_Coords, GetGroundHeight, etc.)
  │   └── TechnoClass::CanCrushCheck (0x5F6CD0)
  ├── Phase 3: Idle path → Process_Movement (0x4B2630)
  │   ├── FootClass::Find_Path (0x4D3920)
  │   ├── CellClass::Can_Enter_Cell_General (0x481A00)
  │   └── MapClass methods (Get_CellClass, Check_Crushable_Obstacle)
  └── Wake animation → AnimClass::Constructor (0x421EA0)
```

---

## 6. Current Rust Implementation Status

### Implemented

| Feature | File | Status |
|---------|------|--------|
| Track system (72 TurnTracks, 16 RawTracks) | `src/sim/movement/drive_track.rs` | Full data + stepping |
| Track point transform (mirror/flip) | `drive_track.rs:44` | Implemented |
| Bridge layer transitions | `src/sim/movement/movement_bridge.rs` | Height-based detection |
| Cell crossing + terrain validation | `src/sim/movement/movement_step.rs` | Cliff, occupancy, terrain |
| Crush/scatter resolution | `src/sim/movement/bump_crush.rs` | Multi-mode occupancy |
| Vehicle rotation (ROT) | `movement_step.rs:158` | Per-tick facing update |
| Occupation marking | `src/sim/movement/bump_crush.rs` | Sub-cell + blocker tracking |
| Locomotor type parsing (CLSID) | `src/rules/locomotor_type.rs` | All 11 CLSIDs |
| Speed/accel/decel | `src/sim/components.rs` | MovementTarget struct |

### Missing or Incomplete

| Feature | Gap | Priority |
|---------|-----|----------|
| Slope visual interpolation (3-frame SLERP) | No VXL matrix generation yet | Medium (rendering) |
| Bridge Z offset initialization chain | Static init functions not ported | High (correct bridge height) |
| Convoy stop propagation | Stop_Moving chain via +0x6C8 linked list | Low |
| Piggyback system | Not needed until Chrono/Teleport locomotor | Low |
| Is_To_Have_Shadow_Override | Shadow position adjustment for bridge ramps | Low (rendering) |
| Sub-tick facing interpolation | `-π/16` per step rotation in shadow/draw | Low (visual polish) |
| Force_Track | Scripted/forced track assignment | Low (AI/scripting) |
| Can_Use_Track | Track chaining validation | Medium (multi-cell turns) |

---

## 7. Doc Conflicts Resolved

### Conflict 1: Slope field at +0x1C
- **OLD:** DRIVE_LOCOMOTION_CLASS.md listed +0x1C as "unknown / possibly ROT"
- **VERIFIED:** +0x1C = `cached_slope_index`, read from `CellClass+0x11C` (SlopeIndex)
- **Evidence:** Process decompilation and Force_New_Slope both confirm

### Conflict 2: Wake animation trigger
- **OLD:** Some docs claimed "SpeedType == 2"
- **VERIFIED:** Checks `CellClass.land_type == 2` (LandType::Water), not SpeedType
- **Evidence:** Process decompilation at 0x4B0500

### Conflict 3: Body tilt threshold
- **OLD:** VXL_DRAW_MATRIX report noted tilt angles at `0xB44310` / `0xB43F08` as "runtime-initialized source unknown"
- **NEW FINDING:** The threshold at `0x7E44E8` = `89478.96` (float) makes the dynamic
  tilt path effectively dead code for DriveLocomotionClass. Ground vehicles NEVER enter
  the complex tilt path — only the slope interpolation path is active.
- **Evidence:** Direct memory read of `0x7E44E8`

### Conflict 4: Speed field at +0x4C
- **OLD:** Some reports listed +0x4C as "speed_residual" (used in track stepping budget)
- **VERIFIED:** +0x4C is an 8-byte double `current_speed` (0.0 to 1.0 fractional).
  Stop_Moving clamps to 0.3, Force_Track sets to 1.0.
- **Note:** The track stepping residual is a different field, likely within Process_Drive_Track's locals.

---

## 8. Open Questions

1. **Offset +0x48:** Set to 0 in constructor. Purpose unknown. Might be a Z-coordinate
   component of a third coordinate triplet, or a separate counter. No method reads it
   in isolation. **Confidence: LOW**

2. **Two owner pointers (+0x08 vs +0x0C):** Both are set to 0 in LocomotionClass
   constructor. Both are used for vtable calls on the owning FootClass. +0x08 is used
   in Set_Destination for high-index vtable calls (0x37C+), +0x0C is used in
   Apply_Track_Delta for lower-index calls (0xF0-0xF4). May be the same pointer
   stored twice, or different interface views of the same object. **Confidence: MEDIUM**

3. **Offset +0x18 (previous_slope):** Force_New_Slope sets this to the same value as
   cached_slope (+0x1C). The Draw_Matrix slope transition reads +0x1C but not +0x18
   directly. The Process function may use +0x18 to detect slope changes. Exact role
   in the transition logic needs Process decompilation at full detail. **Confidence: MEDIUM**

4. **Runtime globals at 0x8A07xx:** `g_TileDivisor`, `g_TileAngleCurrent`, and related
   values are computed at map load from tile geometry. Exact initialization call chain
   not traced. **Confidence: MEDIUM**

5. **Short track selection (use_short_track at +0x60):** Constructor inits to 0.
   Can_Use_Track checks it. Never seen set to 1 in any decompiled method. May be
   always false in practice. **Confidence: LOW**

---

## Sources

**Ghidra functions decompiled (28 functions):**
- `0x004AF3E0` StoreTileHeight
- `0x004AF400` InitHeightStep_A
- `0x004AF440` ComputeFromHeightStep
- `0x004AF470` ComputeBridgeRenderOffset
- `0x004AF4A0` ComputeBridgeZOffset
- `0x004AF500` InitHeightStep2
- `0x004AF540` Constructor
- `0x004AF610` Piggybacker_CLSID
- `0x004AF8E0` Begin_Piggyback
- `0x004AF930` End_Piggyback
- `0x004AF970` Is_Ok_To_End
- `0x004AFB40` Force_New_Slope
- `0x004AFB80` ILocomotion_Is_Moving
- `0x004AFC20` Is_Moving_Now
- `0x004AFC90` Destination
- `0x004AFCC0` Head_To_Coord
- `0x004AFD40` Set_Destination
- `0x004AFE00` Stop_Moving
- `0x004AFF60` Draw_Matrix
- `0x004B0410` Shadow_Matrix
- `0x004B04D0` Update_Facing_From_Type
- `0x004B0AD0` Apply_Track_Delta
- `0x004B0C40` Force_Track
- `0x004B0EF0` Do_Turn
- `0x004B4780` Transform_Track_Coords
- `0x004B4820` In_Which_Layer
- `0x004B4890` Stop_And_Scatter
- `0x004B48D0` Mark_All_Occupation_Bits
- `0x004B4920` Is_To_Have_Shadow_Override
- `0x004B4B00` Can_Use_Track
- `0x004B4C60` Get_Status
- `0x004B4C70` Acquire_Hunter_Seeker_Target
- `0x004B4C80` Is_Surfacing
- `0x004B4CD0` Is_Piggybacking
- `0x0055A6C0` LocomotionClass::Constructor (base class)
- `0x004CADE0` Sin_Lookup_Table4096
- `0x0055B120` LocomotionClass::Build_Shadow_Matrix (estimated)

**Memory inspected:**
- `0x7E6238`, `0x7E1730`, `0x7E1740`, `0x7E6240`, `0x7E44E8`, `0x7E4408`,
  `0x7E48F0`, `0x7E3548`, `0x8A0778`, `0x8A0780`, `0x8A0790`

**Existing docs cross-referenced:**
- `DRIVE_LOCOMOTION_CLASS.md`
- `DRIVE_LOCOMOTION_PROCESS_ANALYSIS.md`
- `DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md`
- `PROCESS_DRIVE_TRACK_DECOMPILATION.md`
- `DRIVE_TRACK_SYSTEM.md`
- `VXL_DRAW_MATRIX_GHIDRA_REPORT.md`
- `SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md`
- `LOCOMOTION_MATH_AND_CONSTANTS.md`
- `BRIDGE_SYSTEM.md`
- `CELL_OCCUPATION_MARKING_GHIDRA_REPORT.md`

**INI files checked:** `rulesmd.ini`, `rules.ini`, `artmd.ini`
