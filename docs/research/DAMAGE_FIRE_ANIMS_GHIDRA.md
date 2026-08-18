# DamageFireAnims — Complete Ghidra Deep Dive

Source: Direct decompilation of gamemd.exe via Ghidra MCP
Functions: `0x0043c0d0`, `0x0043b5e0`, `0x006d2070`, `0x0045ec90`, `0x0045eca0`,
`0x00421ea0`, `0x0065c7e0`, `0x005f5c60`

---

## Overview

DamageFireAnims are a **completely separate system** from the 21-slot building animation array.
They are 8 standalone `AnimClass` instances stored at `BuildingClass + 0x5C8..0x5E7`
(param_1[0x172..0x179]), created from `DamageFireOffset` positions in art.ini and
`DamageFireTypes` anim types from rules.ini `[General]`.

---

## 1. Data Sources

### rules.ini / rulesmd.ini — `[General]` section

```ini
DamageFireTypes=FIRE01,FIRE02,FIRE03  ; Fires that can spring up on damaged buildings
```

Parsed at `0x0066d530` (RulesClass::ReadGeneral, 18,793 bytes):
- Read via `FUN_00528a10` (INI key reader) with key `"DamageFireTypes"`
- Comma-separated list tokenized via `strtok` (`FUN_007c9cc2`)
- Each token looked up via `FUN_00428b80` (FindAnimType by name)
- Stored in a DynamicVector at `Rules + 0x2A4` (pointer array) with count at `Rules + 0x2B0`

### art.ini / artmd.ini — Per-building section

```ini
[GAWEAP]                    ; Allied War Factory
DamageFireOffset0=-26,27    ; pixel offset from building origin
DamageFireOffset1=-2,-57
DamageFireOffset2=22,50
; Up to DamageFireOffset7 (max 8 positions)
```

Stored in `BuildingTypeClass` at offsets `0x15D8..0x1618`:
- 8 entries, 8 bytes each (2 × int32: X pixel, Y pixel)
- Stride: 8 bytes per entry
- Sentinel value: `DAT_0089C848, DAT_0089C84C` (0,0 or invalid coords) = no fire at this slot
- Total: 8 × 8 = 64 bytes

### FIRE01 / FIRE02 / FIRE03 animation properties (from artmd.ini)

```ini
[FIRE01]
Rate=450           ; ms per frame
LoopCount=-1       ; infinite loop
StartSound=BuildingFireBig

[FIRE02]
Rate=450
LoopCount=-1
StartSound=BuildingFireBig

[FIRE03]
Rate=450
LoopCount=-1
StartSound=BuildingFireMed    ; smaller fire sound
```

---

## 2. Storage Layout

### BuildingClass fire anim pointers

| Offset (hex) | Offset (param_1 index) | Type | Description |
|---|---|---|---|
| `+0x5C8` | `[0x172]` | AnimClass* | DamageFireAnim slot 0 |
| `+0x5CC` | `[0x173]` | AnimClass* | DamageFireAnim slot 1 |
| `+0x5D0` | `[0x174]` | AnimClass* | DamageFireAnim slot 2 |
| `+0x5D4` | `[0x175]` | AnimClass* | DamageFireAnim slot 3 |
| `+0x5D8` | `[0x176]` | AnimClass* | DamageFireAnim slot 4 |
| `+0x5DC` | `[0x177]` | AnimClass* | DamageFireAnim slot 5 |
| `+0x5E0` | `[0x178]` | AnimClass* | DamageFireAnim slot 6 |
| `+0x5E4` | `[0x179]` | AnimClass* | DamageFireAnim slot 7 |

### BuildingTypeClass fire offset data

| Offset (hex) | Size | Description |
|---|---|---|
| `+0x15D8` | 4 | DamageFireOffset0.X (pixel) |
| `+0x15DC` | 4 | DamageFireOffset0.Y (pixel) |
| `+0x15E0` | 4 | DamageFireOffset1.X |
| `+0x15E4` | 4 | DamageFireOffset1.Y |
| ... | ... | ... (stride 8 per entry) |
| `+0x1610` | 4 | DamageFireOffset7.X |
| `+0x1614` | 4 | DamageFireOffset7.Y |

Formula: `TypeClass + 0x15D8 + (slotIndex * 8)` for X, `+ 4` for Y.

---

## 3. Creation — `BuildingClass::CreateDamageFireAnims` (`0x0043c0d0`)

460 bytes, 132 instructions, cyclomatic complexity 18.

### Pseudocode

```c
void BuildingClass::CreateDamageFireAnims(this)
{
    // Get count of available fire anim types
    int fireTypeCount = Rules->damageFireTypeCount_0x2B0;  // DAT_008871e0 + 0x2B0
    if (fireTypeCount == 0) return;

    // Pick a random starting index into the DamageFireTypes array
    int animTypeIndex = Random(0, fireTypeCount - 1);  // FUN_0065c7e0

    // Iterate all 8 fire anim slots
    for (int slot = 0; slot < 8; slot++) {
        int* offsetData = &this->type_0x520->damageFireOffsets[slot];  // TypeClass + 0x15D8 + slot*8
        int offsetX = offsetData[0];
        int offsetY = offsetData[1];

        // Check sentinel: if X,Y == (0,0) sentinel, stop
        if (offsetX == SENTINEL_X && offsetY == SENTINEL_Y) {
            return;  // No more fire positions defined — EARLY EXIT
        }

        // Skip if slot already has an anim
        if (this->fireAnims[slot] != NULL) {  // param_1[0x172 + slot]
            return;  // EARLY EXIT — all subsequent slots assumed filled too
        }

        // Convert pixel offset to world coordinates
        // FUN_006d2070: takes (X_pixel, Y_pixel), applies isometric transform via
        // FUN_005afb80 (matrix multiply), then FUN_007c5f00 (float-to-int conversion)
        CoordStruct screenOffset;
        IsometricPixelToWorld(&screenOffset, offsetData);  // FUN_006d2070

        // Get building center position in world coords
        CoordStruct buildingPos;
        this->GetCoords(&buildingPos);  // vtable+0xAC

        // Sum: fire position = building center + converted pixel offset
        CoordStruct firePos;
        firePos.X = screenOffset.X + buildingPos.X;
        firePos.Y = screenOffset.Y + buildingPos.Y;
        firePos.Z = screenOffset.Z + buildingPos.Z;  // screenOffset.Z = 0

        // Allocate AnimClass (0x1C8 = 456 bytes)
        AnimClass* anim = new AnimClass(
            Rules->damageFireTypes_0x2A4[animTypeIndex],  // AnimTypeClass*
            firePos,
            0,        // delay
            1,        // loop
            0x600,    // flags: visible + attached
            0,        // facing
            0         // z-adjust
        );

        if (anim != NULL) {
            // Store in slot
            this->fireAnims[slot] = anim;  // param_1[0x172 + slot] = anim

            // Calculate Z-offset for vertical positioning
            // Foundation height: DAT_00819310[type->foundationType_0xEF0]
            // Foundation width:  DAT_008192B8[type->foundationType_0xEF0]
            int foundationHeight = GetFoundationHeight(0);  // FUN_0045eca0
            int foundationWidth  = GetFoundationWidth();     // FUN_0045ec90

            // Z-offset formula: ((offsetY + (height + width) * -15) * 3 / 2) - 10
            // The -15 comes from HEIGHT_STEP (CellSizeY / 2 = 30 / 2 = 15)
            int zOffset = ((offsetData[1] + (foundationHeight + foundationWidth) * -15) * 3 >> 1) - 10;

            // Clamp: if positive, set to 0 (fires always draw behind/below)
            if (zOffset > 0) zOffset = 0;
            anim->zDrawOffset_0x100 = zOffset;

            // Random starting frame for visual variety
            // AnimTypeClass+0x2C0 = total frame count
            int frameCount = anim->type->frameCount_0x2C0;
            if (frameCount > 0) {
                anim->currentFrame_0xAC = Random(0, frameCount - 1);
            }

            // Cycle to next fire type (wraps around)
            animTypeIndex++;
            if (animTypeIndex >= fireTypeCount) {
                animTypeIndex = 0;
            }
        }
    }
}
```

### Key Constants

| Constant | Address | Value | Purpose |
|---|---|---|---|
| DamageFireTypes array | `Rules + 0x2A4` | AnimTypeClass*[] | Array of fire anim types |
| DamageFireType count | `Rules + 0x2B0` | int | Number of fire anim types (typically 3) |
| Foundation width table | `DAT_008192B8` | int[] | Indexed by foundation enum |
| Foundation height table | `DAT_00819310` | int[] | Indexed by foundation enum |
| Foundation enum | `TypeClass + 0xEF0` | int | Foundation type index |
| HasBib flag | `TypeClass + 0x1570` | byte | If set, height += 1 |
| Sentinel X | `DAT_0089C848` | int | "No fire here" marker |
| Sentinel Y | `DAT_0089C84C` | int | "No fire here" marker |
| HEIGHT_STEP | hardcoded | 15 | Pixel height per cell level |
| AnimClass size | | 0x1C8 (456) | Standard anim allocation |
| Anim flags | | 0x600 | Visible + attached-to-building |

### Z-Offset Formula

```
zOffset = ((DamageFireOffset_Y + (foundationH + foundationW) × -15) × 3/2) - 10
if zOffset > 0: zOffset = 0
```

This ensures fire anims draw at the correct depth relative to the building.
The `× 3/2` is the isometric Y-to-depth conversion factor.
The `-10` provides a small extra push behind the building.
The `foundationH + foundationW` × `-15` accounts for the building's footprint height.

### Random Frame Start

Each fire anim starts at a random frame so multiple fires on the same building
aren't synchronized. This gives a more natural, chaotic look.

### Early Exit Behavior

The function uses **early return** semantics:
- If a slot's offset matches the sentinel → stop (no more defined offsets)
- If a slot is already occupied → stop (building already has fires)

This means fires are created **all at once** when the function is first called,
not one-by-one over time.

---

## 4. Destruction — Building OnDestroyed (`0x445880`)

When a building is destroyed, the 8 fire anims are cleaned up:

```c
// At offset 0x445880 in FUN_00445880:
int** fireSlots = &this->fireAnims_0x5C8;  // param_1 + 0x172
for (int i = 0; i < 8; i++) {
    if (*fireSlots != NULL) {
        (*fireSlots)->vtable->Uninit();  // vtable+0xF8 (AnimClass::Uninit)
        *fireSlots = NULL;
    }
    fireSlots++;
}
```

Note: uses `vtable+0xF8` (Uninit/Remove), NOT `vtable+0x20` (Destroy) — the fire
anims are removed without the normal destruction cascade.

---

## 5. One-Shot Fire Creation — `BuildingClass::CreateFireAnim` (`0x0043b5e0`)

Separate from the 8-slot DamageFireAnim system, this creates a single fire anim at
the building's exact center position. Called from the damage handler `FUN_006cc390`
when a building takes a big hit.

```c
AnimClass* BuildingClass::CreateFireAnim(this, AnimTypeClass* animType)
{
    // Look up anim type by name
    int index = FindAnimType(animName);  // FUN_00427CB0
    AnimTypeClass* type = AnimTypes[index];  // DAT_008B4154[index]

    // Get building center
    CoordStruct pos;
    this->GetCoords(&pos);  // vtable+0x48

    // Create anim at building center
    AnimClass* anim = new AnimClass(type, pos, 0, 1, 0x600, 0, 0);

    // Set draw offsets from building
    anim->SetDrawOffset(/*from return addr*/);  // FUN_00424C90
    anim->SetZAdjust(animType);                 // FUN_00424CA0

    // Mark as visible
    anim->isVisible_0x19D = 1;

    return anim;
}
```

This is a one-shot explosion/fire, not the persistent DamageFireAnims.

---

## 6. Pixel-to-World Conversion — `FUN_006d2070`

Converts `DamageFireOffset` pixel coords to world lepton coords:

```c
void IsometricPixelToWorld(CoordStruct* out, int* pixelXY)
{
    float vec[3];
    vec[0] = (float)pixelXY[0];   // X pixel
    vec[1] = (float)pixelXY[1];   // Y pixel
    vec[2] = 0.0f;                // Z = 0

    // Apply inverse isometric matrix
    float* result = MatrixMultiply(vec);  // FUN_005AFB80

    // Convert float to int (lepton coords)
    out->X = FloatToInt(result[0]);  // FUN_007C5F00
    out->Y = FloatToInt(result[1]);  // FUN_007C5F00
}
```

The matrix at `FUN_005AFB80` is the inverse isometric transform that converts
screen-space pixel offsets back to world-space lepton coordinates.

---

## 7. When Are DamageFireAnims Created?

The function `0x43c0d0` is called from `BuildingClass::Update @ 0x0043FB20` when
the cached damage-fire state at `this+0x5E8` changes from false to true. It is an
AI/update-time spawn path, not render-time lazy creation. The call chain is:

```
BuildingClass::Update (0x0043FB20)
  -> choose threshold from BuildingType+0x157B:
       0 => Rules+0x1700 ConditionYellow
       nonzero => Rules+0x1708 ConditionRed
  -> damaged = GetHealthRatio() <= threshold
  -> if damaged changed and is now true:
       BuildingClass::CreateDamageFireAnims (0x0043C0D0)
```

The fires are created once when the update-time damaged-state transition is
detected. The early-exit check (`if slot already occupied -> return`) prevents
duplicate fire anim creation for occupied fire slots.

The health check is in `BuildingClass::Update`: `BuildingType+0x157B == 0` selects
`ConditionYellow`; nonzero selects `ConditionRed`.

---

## 8. Relationship to DamageParticleSystems

In addition to DamageFireAnims, buildings also emit particle effects when damaged:

```ini
; In rules.ini per building type:
DamageParticleSystems=SparkSys,SmallGreySSys
```

These are **separate** from both the 21-slot system and the DamageFireAnim system.
They are particle system instances that emit sparks and grey smoke independently.

---

## 9. Implementation Guide for Rust Engine

### What to parse from INI

**From rules.ini `[General]`:**
```rust
struct DamageFireConfig {
    fire_types: Vec<String>,  // DamageFireTypes=FIRE01,FIRE02,FIRE03
}
```

**From art.ini per building section:**
```rust
struct DamageFireOffsets {
    offsets: Vec<(i32, i32)>,  // DamageFireOffset0..7, X,Y pixel pairs
    // Max 8 entries, stop at first missing key
}
```

### When to spawn

1. When building health drops below the selected threshold: `ConditionYellow` if
   `BuildingType+0x157B == 0`, otherwise `ConditionRed`
2. Create all fire anims at once (not progressively)
3. Each fire picks a random type from `DamageFireTypes` list, cycling through
4. Each fire starts at a random frame for desynchronized looping

### When to destroy

1. When building health rises above the selected threshold (repair)
2. When building is destroyed
3. When building is sold

### Position calculation

```rust
fn fire_world_position(
    building_center: WorldCoord,
    pixel_offset: (i32, i32),
    foundation_w: i32,
    foundation_h: i32,
) -> (WorldCoord, i32) {
    // Convert pixel offset to world coords via inverse isometric matrix
    let world_offset = pixel_to_world(pixel_offset.0, pixel_offset.1);

    let pos = WorldCoord {
        x: building_center.x + world_offset.x,
        y: building_center.y + world_offset.y,
        z: building_center.z,
    };

    // Z-offset for depth sorting
    let z_offset = ((pixel_offset.1 + (foundation_h + foundation_w) * -15) * 3 / 2) - 10;
    let z_offset = z_offset.min(0);  // clamp to non-positive

    (pos, z_offset)
}
```

### Key differences from 21-slot anims

| Aspect | 21-Slot Anims | DamageFireAnims |
|--------|--------------|-----------------|
| Storage | `+0x55C..0x5AF` (21 ptrs) | `+0x5C8..0x5E7` (8 ptrs) |
| Definition | TypeClass `+0xF4C` (0x44/slot) | TypeClass `+0x15D8` (8 bytes/slot) |
| Anim types | Per-slot in art.ini | Global `DamageFireTypes` from rules.ini |
| Position | Fixed per-slot XY offset | `DamageFireOffset` pixel coords → world |
| Damaged variants | Yes (undamaged/damaged art) | No (same FIRE01/02/03 always) |
| Power toggle | Yes (3 flag bytes per slot) | No |
| Cloaking | Yes (translucency propagated) | No (independent anims) |
| Random start | No (deterministic frame) | Yes (random starting frame) |
| Cleanup on destroy | `vtable+0x20` (Destroy) | `vtable+0xF8` (Uninit) |
| Creation trigger | Various (placement, state) | Health < ConditionYellow |
| Max count | 21 (fixed) | 8 (max, from DamageFireOffset keys) |
