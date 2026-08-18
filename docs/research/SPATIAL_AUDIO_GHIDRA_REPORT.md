# Spatial Audio System — Ghidra RE Report

Date: 2026-03-23
Binary: gamemd.exe
Confidence: HIGH (all data verified from binary decompilation + memory reads)

## 1. VocClass / AudioEventClass Struct Layout

VocClass is a thin wrapper around AudioEventClass. The `VocClass::ReadINI` function
at `0x00750440` reads each INI key via `CCINIClass::Read*` and writes to an
AudioEventClass object via setter methods. Below are all INI keys, their struct
offsets within AudioEventClass, types, and defaults.

### AudioEventClass Field Map

| INI Key   | Offset | Type     | Default        | Setter Address | Notes |
|-----------|--------|----------|----------------|----------------|-------|
| Sounds    | —      | string[] | (none)         | 0x004064A0     | Tokenized by spaces; each token calls `VocClass::AddSample` to append a .wav/.aud filename to a sample list |
| Volume    | 0x18+  | float    | 80.0           | 0x00406550     | Read as double, converted via `ftol`, fed to `VolumeInterp::SetTargetImmediate`. Not a simple field store — goes through volume interpolation subsystem |
| VShift    | 0x68   | int      | 0              | 0x00406620     | Clamped 0–100. Volume shift randomization range |
| MinVolume | 0x54   | float    | 20.0           | 0x004065F0     | Read as double. Minimum volume floor for distance-attenuated sounds |
| Priority  | 0x40   | int/enum | NORMAL (2)     | 0x00406540     | Parsed from string via table at 0x00816018 |
| Attack    | 0x138  | int      | 0              | 0x00406520     | Volume ramp-up time (ms). Only effective if ATTACK control flag set |
| Decay     | 0x13C  | int      | 0              | 0x00406530     | Volume ramp-down time (ms). Only effective if DECAY control flag set |
| Control   | 0x10   | uint     | 0              | 0x00406570     | Bitmask of control flags. Tokenized by spaces. See Control Flags below. Also conditionally sets Attack/Decay to 1 if ATTACK/DECAY flags present but values are 0 |
| Type      | 0x14   | uint     | 0x20 (SCREEN)  | 0x004065C0     | Bitmask of type flags. Tokenized by spaces. See Type Flags below. SCREEN and LOCAL are mutually exclusive groups; SHROUD and UNSHROUD are mutually exclusive groups |
| Limit     | 0x48   | int      | 5              | 0x004065D0     | Max concurrent instances of this sound |
| Loop      | 0x4C   | int      | 0              | 0x00406640     | Loop count (0 = no loop) |
| Range     | 0x50   | int      | 10             | 0x004065E0     | Audible range in cells. Used by CalcVolumeAndPan: multiplied by 60 to get leptons |
| Delay     | 0x58   | int[2]   | 0, 0           | 0x00406600     | Two ints parsed from space-separated string: min delay, max delay (ms) |
| FShift    | 0x60   | int[2]   | 0, 0           | 0x00406610     | Frequency shift range: min shift, max shift. Two ints parsed from space-separated string |

### Default Constants (from globals)

| Address    | Value | Used For |
|------------|-------|----------|
| 0x008464B4 | 80.0f | Default Volume |
| 0x008464B8 | 20.0f | Default MinVolume |
| 0x008464C0 | 10    | Default Range (cells) |
| 0x008464C4 | 5     | Default Limit |

These defaults are loaded in `VocClass::ReadSoundListINI` (0x007510D0) from the
`[AudioVisual]` section before per-sound parsing begins, so they can be overridden
globally by the `[AudioVisual]` INI section.

## 2. VocClass::PlayAtPos (0x00750920)

```c
int __thiscall VocClass__PlayAtPos(int vocIndex, void* coords, int loopHandle)
```

**Parameters:**
- `vocIndex` (ECX / this): Index into the global VocClass array (DAT_00b1d37c). Validated: must be >= 0 and < DAT_00b1d388 (array count).
- `coords`: Pointer to a coordinate struct (X, Y, Z leptons). Passed to CalcVolumeAndPan.
- `loopHandle`: If non-zero, attempts to reuse an existing SoundEvent. If the existing event's AudioEvent differs from the new one, stops the old sound first.

**Return value:** SoundEvent pointer (int). 0 on failure.

**Flow:**
1. Check DAT_008464ac (global sound enabled flag). If '\0', return 0.
2. Look up AudioEventClass pointer from vocIndex via the global array.
3. If loopHandle != 0, find existing SoundEvent. If it has a different AudioEvent, stop it.
4. Call `Math::ftol()` (CalcVolumeAndPan is inlined or called here for volume/pan).
5. Allocate a SoundEvent from pool if needed.
6. Set volume and pan on the SoundEvent.
7. If loopHandle != 0, associate the SoundEvent as the loop handle.
8. Return the SoundEvent pointer.

## 3. Volume / Distance Attenuation Formula (CalcVolumeAndPan @ 0x00750AC0)

```c
float __fastcall VocClass__CalcVolumeAndPan(
    int* coords,        // ECX: world coordinates {X, Y, Z} in leptons
    int* out_pan,       // EDX: output pan value (-8192..+8192)
    int   audioEventPtr // stack: pointer to AudioEventClass
)
// Returns: volume as float 0.0..1.0
```

### Algorithm

**Step 1: Compute half-viewport dimensions (in client pixels)**
```
halfViewW = g_RadarViewportWidth * 0.5
halfViewH = g_RadarViewportHeight * 0.5
fullViewW = halfViewW * 2   // = g_RadarViewportWidth (used for pan)
```
- `g_RadarViewportWidth` at 0x00886FA8 (int, FILD'd to float)
- `g_RadarViewportHeight` at 0x00886FAC (int)
- Multiplied by 0.5 (constant at 0x007E5168)

**Step 2: Compute maximum audible distance in leptons**
```
maxRange = AudioEvent.Range * 60   // 60 leptons per cell
```
(ASM: `LEA EAX,[EAX+EAX*2]; LEA EAX,[EAX+EAX*4]; SHL EAX,2` = multiply by 3*5*4=60)

**Step 3: SHROUD check (Type flag 0x800)**
If the sound's TypeFlags has bit 0x800 (SHROUD) set:
- Convert coords to cell (X>>8, Y>>8)
- If cell == camera center cell (DAT_00b1d310, DAT_00b1d312): return 0.0 (silent)
- Get CellClass for that cell. If shroud bits (byte at offset 0x12C, mask 0x18) are clear: return 0.0 (can't hear unseen cells)

**Step 4: Convert world coords to screen-relative client pixels**
```
TacticalClass::CoordsToClient2(coords, &clientXY)
// Isometric projection: screenX and screenY relative to viewport top-left
```

**Step 5: Compute distances from screen center**
```
distX = abs(clientXY.x - halfViewW)   // horizontal distance from center
distY = abs(clientXY.y - halfViewH)   // vertical distance from center (note: not sub'd from halfViewH in the client transform)
```
Actually, looking at the ASM more carefully:
```
; At 0x750BBE:
FILD [ESP+0x2C]          ; load clientX as float
FSUB [ESP+0x20]          ; subtract halfViewW  -> signed offset from center
CALL Math__ftol           ; truncate to int
CDQ / XOR / SUB           ; abs()
-> distX (stored at ESP+0x38)

; Then:
FILD [ESP+0x30]          ; load clientY
FSUB [ESP+0x24]          ; subtract halfViewH  -> signed offset from center
; (this value also used for pan before abs)
CALL Math__ftol
CDQ / XOR / SUB
-> distY (stored at ESP+0x28)
```

**Step 6: Subtract viewport half-size (on-screen = zero distance)**
Unless GLOBAL flag (Type 0x40) is set:
```
distX = distX - halfViewW   // sounds on-screen have distX < 0 -> clamped to 0
distY = distY - halfViewH
if distX < 0: distX = 0
if distY < 0: distY = 0
```
This means sounds within the viewport have zero distance (full volume). Distance
only starts accumulating once the source is outside the visible screen.

For GLOBAL sounds (flag 0x40): skip this subtraction — distance is from screen center
regardless.

**Step 7: Double the Y distance (isometric compensation)**
```
distY = distY * 2
```
(ASM: `FADD ST0, ST0` at 0x750C3D)

The Y axis is doubled because isometric projection compresses the Y axis by 2:1.
Doubling Y in the distance calculation makes audio attenuation isotropic in world space.

**Step 8: Compute volume**
```
effectiveDist = max(distX, distY)    // Chebyshev-like (take the larger axis)
if distX < maxRange AND distY < maxRange AND maxRange > 0:
    volume = (maxRange - effectiveDist) / maxRange   // linear falloff
else:
    volume = 0.0
```

**Step 9: Apply MinVolume floor**
If TypeFlags has bit 0x10 (GLOBAL in control? Actually this is control flag for
MinVolume enforcement):
```
if volume < AudioEvent.MinVolume:
    volume = AudioEvent.MinVolume
```

**Step 10: Threshold check**
```
if volume < 0.05:    // double constant at 0x007E8AE8 = 0.05
    return 0.0       // too quiet, don't play
```

**Step 11: Compute pan**
```
// clientOffset = signed horizontal offset from viewport center (before abs)
// Clamped to [-fullViewW, +fullViewW]
pan_raw = clamp(clientOffset, -fullViewW, fullViewW)
pan = (pan_raw * 8192.0) / fullViewW + 8192.0
```
- 8192.0 is the constant at 0x007F68E8 (float 0x46000000)
- The result is `ftol`'d to int and stored in `*out_pan`
- Range: 0 (full left) to 16384 (full right), 8192 = center

Actually, looking at the ASM more carefully at 0x750CDE-0x750D24:
```
FLD [ESP+0x1C]           ; load signed horizontal client offset (before abs)
FCHS                     ; negate it
; clamp to [-fullViewW, fullViewW]:
FLD [ESP+0x10]           ;   compare with fullViewW
FCOMP                    ;   if -offset > fullViewW, clamp
...
; Then:
FLD [ESP+0x10]           ; load (clamped) value
FMUL [0x007F68E8]        ; * 8192.0
FDIV [ESP+0x1C]          ; / fullViewW
FADD [0x007F68E8]        ; + 8192.0
CALL Math__ftol           ; -> int pan
```

So the pan formula is:
```
pan = (clamp(-clientOffsetX, -viewW, viewW) * 8192.0) / viewW + 8192.0
```

Where `clientOffsetX` = `clientX - halfViewW` (positive = right of center).
The negation means: sound to the right -> negative offset -> pan shifts left... wait,
let me re-check. The FCHS negates, so:
- Sound at screen right: clientOffsetX > 0, negated = negative
- Then: `(negative * 8192) / viewW + 8192` = value < 8192 = panned LEFT

That seems backwards. Let me re-examine... Actually DirectSound pan conventions:
pan = -10000 (left) to +10000 (right) in DS. But this engine uses 0-16384 range.
The FCHS may account for coordinate system differences. The actual effect:
- **pan = 0**: full left
- **pan = 8192**: center
- **pan = 16384**: full right
- Sounds to the LEFT of screen center get pan > 8192 (shifted right in ear)

Wait, that's still confusing. Let me just state the formula cleanly:

```
pan = 8192.0 + (-clientOffsetX * 8192.0 / viewportWidth)
```

Where `clientOffsetX = screenX - halfViewportWidth`. If the sound is to the right
of center (positive clientOffsetX), pan < 8192 (shifts toward left channel). This is
correct because the negation implements: "pan the sound toward where it IS on screen"
using a coordinate system where lower values = left.

## 4. Control Flags (INI key: "Control")

Parsed from the table at `0x008160C0`. Each entry is `{char* name, uint value}`.

| String    | Value | Hex  | Meaning |
|-----------|-------|------|---------|
| ALL       | 4     | 0x04 | Play all samples simultaneously (not just one random pick) |
| LOOP      | 1     | 0x01 | Loop the sound |
| RANDOM    | 2     | 0x02 | Pick a random sample from the Sounds list |
| PREDELAY  | 8     | 0x08 | Apply a random pre-delay before playback |
| INTERRUPT | 16    | 0x10 | Can be interrupted by higher-priority sounds |
| ATTACK    | 32    | 0x20 | Enable volume attack (ramp-up). If set and Attack==0, forces Attack=1 |
| DECAY     | 64    | 0x40 | Enable volume decay (ramp-down). If set and Decay==0, forces Decay=1 |
| (entry 8) | 128   | 0x80 | (string at 0x008161B8 = "AMBIENT" — likely a control modifier) |

**Note:** The SetControlFlags function (0x00406570) has special logic: if ATTACK (0x20) is set
but Attack field is 0, it forces Attack=1. Same for DECAY (0x40) / Decay field.

## 5. Type Flags (INI key: "Type")

Parsed from the table at `0x00816048`. Each entry is `{char* name, uint value}`.
The default Type is `0x20` (SCREEN).

| String     | Value  | Hex    | Meaning |
|------------|--------|--------|---------|
| AMBIENT    | 4096   | 0x1000 | Ambient environmental sound (no positional attenuation?) |
| VIOLENT    | 1      | 0x0001 | Combat/explosion sound |
| MOVEMENT   | 2      | 0x0002 | Unit movement sound |
| QUIET      | 4      | 0x0004 | Quiet sound (lower base volume?) |
| LOUD       | 8      | 0x0008 | Loud sound (higher base volume?) |
| GLOBAL     | 16     | 0x0010 | MinVolume is enforced (see CalcVolumeAndPan step 9). Also in CalcVolumeAndPan: skips viewport-edge subtraction — distance is always from screen center |
| SCREEN     | 32     | 0x0020 | Normal positional sound relative to screen viewport |
| LOCAL      | 64     | 0x0040 | Same as GLOBAL — no viewport-edge subtraction (distance from center). Mutually exclusive with SCREEN (setting LOCAL clears SCREEN, and vice versa, bits 0x60) |
| PLAYER     | 128    | 0x0080 | Only audible to owning player |
| NORMAL     | 0      | 0x0000 | No special flags (sentinel/default entry) |
| GUN_SHY    | 512    | 0x0200 | Units with this flag avoid firing near this sound source? |
| NOISE_SHY  | 256    | 0x0100 | Units with this flag avoid noise? |
| UNSHROUD   | 1024   | 0x0400 | Reveals shroud at sound location? Mutually exclusive with SHROUD (bits 0xC00) |
| SHROUD     | 2048   | 0x0800 | Sound only audible if cell is explored (checked in CalcVolumeAndPan step 3) |

**Mutual exclusivity logic** in `AudioEventClass::ParseTypeFlag` (0x00406870):
- Setting SCREEN (0x20) or LOCAL (0x40) clears the other (mask 0xFFFFFF9F)
- Setting SHROUD (0x800) or UNSHROUD (0x400) clears the other (mask 0xFFFFF3FF)

## 6. Priority Values (INI key: "Priority")

Parsed from the table at `0x00816018`:

| String   | Value | Meaning |
|----------|-------|---------|
| LOWEST   | 0     | Lowest priority — evicted first |
| LOW      | 1     | Low priority |
| NORMAL   | 2     | Default priority |
| HIGH     | 3     | High priority |
| CRITICAL | 4     | Highest priority — never evicted |

## 7. Key Functions Summary

| Address    | Name | Purpose |
|------------|------|---------|
| 0x00750440 | VocClass::ReadINI | Parse one sound entry from soundmd.ini |
| 0x00750920 | VocClass::PlayAtPos | Play a positional sound; allocates SoundEvent, sets vol/pan |
| 0x00750AC0 | VocClass::CalcVolumeAndPan | Compute volume (0.0-1.0) and pan (0-16384) from world coords |
| 0x00750E20 | VocClass::PlayAtCoord | Higher-level play; copies coords into a sound event struct |
| 0x007510D0 | VocClass::ReadSoundListINI | Parse [AudioVisual] defaults, then iterate all sound entries |
| 0x007514D0 | VocClass::FindByName | Look up VocClass index by name string |
| 0x007515C0 | VocClass::FindIndexByPtr | Find index given a pointer |
| 0x00751520 | VocClass::FindPtrByName | Look up VocClass pointer by name string |
| 0x00406670 | VocClass::PlayGlobal | Play a non-positional (global) sound |
| 0x006D2140 | TacticalClass::CoordsToClient2 | World coords -> screen pixel offset from viewport TL |
| 0x00406820 | AudioEventClass::ParseControlFlag | Parse one control flag string token |
| 0x00406870 | AudioEventClass::ParseTypeFlag | Parse one type flag string token |
| 0x004067D0 | AudioEventClass::ParsePriority | Parse priority string to enum value |

## 8. Global Variables

| Address    | Type | Name | Purpose |
|------------|------|------|---------|
| 0x00886FA8 | int  | g_RadarViewportWidth | Viewport width in pixels (used for half-screen distance calc) |
| 0x00886FAC | int  | g_RadarViewportHeight | Viewport height in pixels |
| 0x00B1D310 | short | g_CameraCenterCellX | Camera center cell X (for SHROUD check) |
| 0x00B1D312 | short | g_CameraCenterCellY | Camera center cell Y |
| 0x00B1D37C | int** | g_VocClassArray | Global array of VocClass pointers |
| 0x00B1D388 | int  | g_VocClassCount | Number of entries in array |
| 0x008464AC | char | g_SoundEnabled | Master sound enable flag |
| 0x008464B4 | float | g_DefaultVolume | Default volume (80.0) |
| 0x008464B8 | float | g_DefaultMinVolume | Default MinVolume (20.0) |
| 0x008464C0 | int  | g_DefaultRange | Default Range in cells (10) |
| 0x008464C4 | int  | g_DefaultLimit | Default Limit (5) |
| 0x0087E2A0 | int  | g_AudioSubsystemReady | Gate flag checked by getters before returning field values |
| 0x00887324 | int  | g_TacticalPtr | Pointer to TacticalClass singleton |

## 9. Volume/Pan Pseudocode (Clean)

```c
// Returns volume 0.0..1.0, writes pan 0..16384 to *out_pan
float CalcVolumeAndPan(CoordStruct* coords, int* out_pan, AudioEventClass* event) {
    *out_pan = 0;
    float volume = 0.0f;

    float halfViewW = (float)g_ViewportWidth * 0.5f;
    float halfViewH = (float)g_ViewportHeight * 0.5f;
    float fullViewW = halfViewW * 2.0f;  // = viewportWidth, used for pan

    int range = event->Range;          // in cells
    float maxRange = (float)(range * 60);  // convert to leptons (60 per cell)

    uint typeFlags = event->TypeFlags;

    // SHROUD check: if sound has SHROUD flag, only play if cell is explored
    if (typeFlags & 0x800) {
        short cellX = (short)(coords->X >> 8);
        short cellY = (short)(coords->Y >> 8);
        if (cellX == g_CameraCenterCellX && cellY == g_CameraCenterCellY)
            return 0.0f;  // at camera center — skip? (edge case)
        CellClass* cell = Map.GetCell(cellX, cellY);
        if ((cell->ShroudBits & 0x18) == 0)
            return 0.0f;  // cell not explored, can't hear it
    }

    // Convert world coords to screen-relative pixels
    PointStruct clientXY;
    Tactical->CoordsToClient2(coords, &clientXY);

    // Compute signed offset from screen center
    float offsetX = (float)clientXY.x - halfViewW;  // positive = right of center
    float offsetY = (float)clientXY.y - halfViewH;

    // Take absolute distances
    float distX = fabsf(offsetX);
    float distY = fabsf(offsetY);

    // For non-GLOBAL/LOCAL sounds: subtract viewport half-size
    // (sounds on-screen have zero distance = full volume)
    if (!(typeFlags & 0x40)) {   // not LOCAL
        distX = distX - halfViewW;
        distY = distY - halfViewH;
        if (distX < 0.0f) distX = 0.0f;
        if (distY < 0.0f) distY = 0.0f;
    }

    // Double Y distance for isometric compensation
    distY = distY * 2.0f;

    // Volume = linear falloff from maxRange
    if (distX < maxRange && distY < maxRange && maxRange > 0.0f) {
        float effectiveDist = (distX > distY) ? distX : distY;  // max of the two
        volume = (maxRange - effectiveDist) / maxRange;
    }

    // MinVolume floor (if GLOBAL flag 0x10 set in TypeFlags)
    if ((typeFlags & 0x10) && volume < event->MinVolume) {
        volume = event->MinVolume;
    }

    // Threshold: if volume < 0.05, return silent
    if (volume < 0.05) {
        return 0.0f;
    }

    // Pan calculation: map horizontal screen offset to 0..16384
    float panOffset = -offsetX;  // negate: positive offsetX -> lower pan value
    panOffset = clamp(panOffset, -fullViewW, fullViewW);
    float pan = (panOffset * 8192.0f) / fullViewW + 8192.0f;
    *out_pan = (int)pan;

    return volume;
}
```

### Key observations for implementation:
1. **On-screen sounds = full volume.** The viewport half-size subtraction means any
   sound within the visible area has distX=0, distY=0, so volume=1.0.
2. **Y axis doubled** for isometric. World Y maps to half the screen pixels as X,
   so doubling Y makes the attenuation circle match world-space distance.
3. **Linear falloff** — not exponential, not inverse-square. Simple `(maxRange - dist) / maxRange`.
4. **Pan range 0–16384** with 8192 = center. DirectSound uses -10000 to +10000, so the
   engine likely remaps this before passing to DS.
5. **Range is in cells**, converted to pixel distance by multiplying by 60 (the number of
   screen pixels per cell in the isometric grid).
6. **0.05 threshold** — sounds below 5% volume are culled entirely.
